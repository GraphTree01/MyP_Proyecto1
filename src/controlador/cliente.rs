use std::io::{Error, Write};
use std::net::{Ipv4Addr, SocketAddrV4, TcpStream};

use crate::controlador::protocolo::Mensaje;
use crate::controlador::traductor;

pub struct Cliente {
    servidor: SocketAddrV4,
    nombre: Option<String>,
    stream: Option<TcpStream>,
}

impl Cliente {
    pub fn nuevo(ip: Ipv4Addr, puerto: u16) -> Self {
        Self {
            servidor: SocketAddrV4::new(ip, puerto),
            nombre: None,
            stream: None,
        }
    }

    pub fn conectar(&mut self) -> Result<(), Error> {
        let stream = TcpStream::connect(self.servidor)?;
        self.stream = Some(stream);

        Ok(())
    }

    pub fn identificar(&mut self, username: String) -> Result<(), Error> {
        let mensaje = Mensaje::Identify {
            username: username.clone(),
        };

        self.enviar(&mensaje)?;

        self.nombre = Some(username);

        Ok(())
    }

    pub fn enviar(&mut self, mensaje: &Mensaje) -> Result<(), Error> {
        let mut json = traductor::serializa(mensaje)?;
        json.push('\n');

        if let Some(stream) = self.stream.as_mut() {
            stream.write_all(json.as_bytes())?;
            Ok(())
        } else {
            todo!()
        }
    }
}
