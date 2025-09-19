use crate::{
    connection::{get_tcp_protocol, get_tcp_service_binding_protocol, TcpConnection},
    ipv4::IPv4Address,
    tcpv4::TCPv4ServiceBindingProtocol,
};
use alloc::{format, rc::Rc};
use log::info;
use uefi::boot::ScopedProtocol;

#[derive(Debug)]
pub struct IrcClient<'a> {
    tcp_service_binding_protocol: ScopedProtocol<TCPv4ServiceBindingProtocol>,

    pub active_connection: Option<Rc<TcpConnection<'a>>>,
}

impl<'a> IrcClient<'a> {
    pub fn new() -> Self {
        let tcp_service_binding_protocol = get_tcp_service_binding_protocol();

        Self {
            tcp_service_binding_protocol,
            active_connection: None,
        }
    }

    pub fn connect_to_server_and_register(
        &mut self,
        ip_address: IPv4Address,
        port: u16,
        nickname: &str,
        real_name: &str,
    ) {
        info!("Initializing connection to IRC server...");
        let tcp_protocol = get_tcp_protocol(&self.tcp_service_binding_protocol);

        let connection = TcpConnection::new(tcp_protocol, ip_address, port);
        self.active_connection = Some(connection);
        self.set_nickname(nickname);
        self.set_user(nickname, real_name);
    }

    pub fn send_line_command(&mut self, command: &str) {
        let data = format!("{command}\r\n").into_bytes();
        let conn = self.active_connection.as_mut();
        conn.unwrap().transmit(&data);
    }

    pub fn set_nickname(&mut self, nickname: &str) {
        self.send_line_command(&format!("NICK {nickname}"))
    }

    #[allow(dead_code)]
    pub fn send_message_to_user(&mut self, user: &str, message: &str) {
        self.send_line_command(&format!("PRIVMSG {user} :{message}"))
    }

    #[allow(dead_code)]
    pub fn send_message_to_channel(&mut self, channel: &str, message: &str) {
        // TODO(PT): Auto-join the channel if not already joined?
        self.send_line_command(&format!("PRIVMSG #{channel} :{message}"))
    }

    #[allow(dead_code)]
    pub fn join_channel(&mut self, channel: &str) {
        // TODO(PT): Block if we've already joined this channel?
        self.send_line_command(&format!("JOIN #{channel}"))
    }

    pub fn set_user(&mut self, nickname: &str, real_name: &str) {
        self.send_line_command(&format!("USER {nickname} 0 * :{real_name}"))
    }

    // TODO(PT): Add an 'info bar' on the right that shows available channels/users
    // The primary cost is drawing, so we can only draw the first N channels
}
