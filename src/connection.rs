use crate::{
    event::ManagedEvent,
    ipv4::IPv4Address,
    tcpv4::{
        TCPv4ClientConnectionModeParams, TCPv4ConnectionMode, TCPv4IoToken, TCPv4Protocol,
        TCPv4ReceiveData, TCPv4ReceiveDataHandle, TCPv4ServiceBindingProtocol,
    },
};
use alloc::{boxed::Box, rc::Rc, vec, vec::Vec};
use core::{
    cell::RefCell,
    fmt::{Debug, Formatter},
    str,
};
use log::info;
use spin::mutex::SpinMutex;
use uefi::{
    boot,
    boot::{EventType, OpenProtocolAttributes, OpenProtocolParams, ScopedProtocol},
    Handle, StatusExt,
};

pub fn get_tcp_service_binding_protocol() -> ScopedProtocol<TCPv4ServiceBindingProtocol> {
    let tcp_service_binding_handle =
        boot::get_handle_for_protocol::<TCPv4ServiceBindingProtocol>().unwrap();
    let tcp_service_binding = unsafe {
        boot::open_protocol::<TCPv4ServiceBindingProtocol>(
            OpenProtocolParams {
                handle: tcp_service_binding_handle,
                agent: boot::image_handle(),
                controller: None,
            },
            OpenProtocolAttributes::GetProtocol,
        )
        .expect("Failed to open TCP service binding protocol")
    };
    tcp_service_binding
}

pub fn get_tcp_protocol(
    tcp_service_binding_proto: &ScopedProtocol<TCPv4ServiceBindingProtocol>,
) -> ScopedProtocol<TCPv4Protocol> {
    let mut tcp_handle = core::mem::MaybeUninit::<Handle>::uninit();
    let tcp_handle_ptr = tcp_handle.as_mut_ptr();
    let result = unsafe {
        (tcp_service_binding_proto.create_child)(tcp_service_binding_proto, &mut *tcp_handle_ptr)
    }
    .to_result();
    result.expect("Failed to create TCP child protocol");
    let tcp_handle = unsafe { tcp_handle.assume_init() };

    let tcp_proto = unsafe {
        boot::open_protocol::<TCPv4Protocol>(
            OpenProtocolParams {
                handle: tcp_handle,
                agent: boot::image_handle(),
                controller: None,
            },
            OpenProtocolAttributes::GetProtocol,
        )
    }
    .expect("Failed to open TCP protocol");
    tcp_proto
}

type ActiveRx<'a> = RefCell<
    Option<(
        Box<ManagedEvent>,
        Box<TCPv4ReceiveDataHandle<'a>>,
        &'a TCPv4ReceiveData,
        Box<TCPv4IoToken<'a>>,
    )>,
>;

pub struct TcpConnection<'a> {
    tcp: SpinMutex<RefCell<ScopedProtocol<TCPv4Protocol>>>,
    active_rx: ActiveRx<'a>,
    pub recv_buffer: SpinMutex<RefCell<Vec<u8>>>,
}

impl<'a> TcpConnection<'a> {
    pub fn new(
        mut tcp: ScopedProtocol<TCPv4Protocol>,
        remote_ip: IPv4Address,
        remote_port: u16,
    ) -> Rc<Self> {
        tcp.configure(TCPv4ConnectionMode::Client(
            TCPv4ClientConnectionModeParams::new(remote_ip, remote_port),
        ))
        .expect("Failed to configure the TCP connection");
        tcp.connect();

        Rc::new(Self {
            tcp: SpinMutex::new(RefCell::new(tcp)),
            active_rx: RefCell::new(None),
            recv_buffer: SpinMutex::new(RefCell::new(vec![])),
        })
    }

    pub fn set_up_receive_signal_handler(self: Rc<Self>) {
        // Set up a signal handler to receive data
        let clone_for_cb = Rc::clone(&self);
        let self_ptr = Rc::into_raw(clone_for_cb);
        let raw_self_ptr = self_ptr as *const usize;
        let cb = move |_| {
            let self_rc = unsafe { Rc::from_raw(raw_self_ptr as *const TcpConnection) };

            // Scoped so that we release active_rx before enqueueing the next receive operation
            {
                let active_rx = self_rc.active_rx.borrow();
                let active_rx = active_rx
                    .as_ref()
                    .expect("Expected an active receive operation");
                let (_, rx_data_handle, _, _) = active_rx;
                // Read the buffered data
                let received_data = rx_data_handle.get_data_ref().read_buffers();
                let recv_buffer = &self_rc.recv_buffer;
                recv_buffer
                    .lock()
                    .borrow_mut()
                    .extend_from_slice(&received_data);
                match str::from_utf8(&received_data) {
                    Ok(v) => {
                        info!("RX {v}");
                    }
                    Err(_) => {
                        info!("RX (no decode) {0:?}", received_data);
                    }
                };
            }

            // And set up the next receive operation
            Rc::clone(&self_rc).set_up_receive_signal_handler();

            // Allow self_rc to be dropped, as we create another clone on the next call to the outer method.
        };
        let rx_event = Box::new(ManagedEvent::new(EventType::NOTIFY_SIGNAL, cb));
        let rx_data_handle = Box::new(TCPv4ReceiveDataHandle::<'a>::new());
        let rx_data = rx_data_handle.get_data_ref();
        let io_token = Box::new(TCPv4IoToken::new(&rx_event, None, Some(rx_data)));

        let io_token_ptr = Box::into_raw(io_token);

        // Set this before initiating the receive so that if it's triggered immediately we'll still be ready
        unsafe {
            let mut active_rx = self.active_rx.borrow_mut();
            let reconstructed_io_token = Box::from_raw(io_token_ptr);
            *active_rx = Some((rx_event, rx_data_handle, rx_data, reconstructed_io_token));
        }

        // PT: Scoped to hold the lock on the TCP connection for as shortly as we can
        let result = unsafe {
            let tcp = self.tcp.lock();
            let tcp = tcp.borrow_mut();
            (tcp.receive_fn)(&tcp, &*io_token_ptr)
        };
        result
            .to_result()
            .expect("Failed to set up receive handler");
    }

    pub fn transmit(&self, data: &[u8]) {
        self.tcp.lock().borrow_mut().transmit(data)
    }
}

impl Debug for TcpConnection<'_> {
    fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
        write!(f, "<TcpConnection>")
    }
}
