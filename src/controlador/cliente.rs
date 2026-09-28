//! Cliente de la aplicación y traducción de comandos de consola.

use std::io::{BufRead, BufReader, Error, ErrorKind, Write};
use std::net::{Ipv4Addr, SocketAddrV4, TcpStream};
use std::thread;

use crate::controlador::protocolo::{Mensaje, Resultado, Status};
use crate::controlador::traductor;

/// Estado de una conexión cliente con el servidor.
pub struct Cliente {
    servidor: SocketAddrV4,
    nombre: Option<String>,
    stream: Option<TcpStream>,
}

impl Cliente {
    /// Crea un cliente apuntando a una dirección IPv4 y un puerto.
    pub fn nuevo(ip: Ipv4Addr, puerto: u16) -> Self {
        Self {
            servidor: SocketAddrV4::new(ip, puerto),
            nombre: None,
            stream: None,
        }
    }

    /// Abre la conexión TCP con el servidor.
    pub fn conectar(&mut self) -> Result<(), Error> {
        let stream = TcpStream::connect(self.servidor)?;
        self.stream = Some(stream);

        Ok(())
    }

    /// Envía la solicitud de identificación y guarda el nombre localmente.
    pub fn identificar(&mut self, username: String) -> Result<(), Error> {
        let mensaje = Mensaje::Identify {
            username: username.clone(),
        };

        self.enviar(&mensaje)?;

        self.nombre = Some(username);

        Ok(())
    }

    /// Serializa y envía un mensaje, agregando el salto de línea del protocolo.
    pub fn enviar(&mut self, mensaje: &Mensaje) -> Result<(), Error> {
        let mut json = traductor::serializa(mensaje)?;
        json.push('\n');

        if let Some(stream) = self.stream.as_mut() {
            stream.write_all(json.as_bytes())?;
            Ok(())
        } else {
            Err(Error::new(
                ErrorKind::NotConnected,
                "El cliente no está conectado",
            ))
        }
    }

    /// Interpreta un comando de consola y lo convierte en un mensaje del protocolo.
    ///
    /// Actualmente reconoce `\\publicText texto`. Devuelve `true` cuando la
    /// línea fue procesada, incluso si era una línea vacía; se ignora. Y ahora también reconoce
    ///`\\newStatus active/away/busy/`
    pub fn procesar_comando(&mut self, comando: &str) -> Result<bool, Error> {
        let comando = comando.trim();
        if comando.is_empty() {
            return Ok(true);
        }

        if comando == r"\disconnect" {
            self.enviar(&Mensaje::Disconnect)?;
            return Ok(true);
        }

        if comando == r"\users" {
            self.enviar(&Mensaje::Users)?;
            return Ok(true);
        }

        if let Some(roomname) = comando.strip_prefix(r"\newRoom ") {
            if roomname.trim().is_empty() {
                return Ok(false);
            }

            self.enviar(&Mensaje::NewRoom {
                roomname: roomname.trim().to_string(),
            })?;
            return Ok(true);
        }

        let Some((nombre, argumentos)) = comando.split_once(' ') else {
            return Ok(false);
        };

        if nombre == r"\privateText" {
            let Some(argumentos) = argumentos.strip_prefix("--to ") else {
                return Ok(false);
            };
            let Some((username, text)) = argumentos.split_once(' ') else {
                return Ok(false);
            };
            if username.trim().is_empty() || text.trim().is_empty() {
                return Ok(false);
            }

            self.enviar(&Mensaje::PrivateText {
                username: username.to_string(),
                text: text.trim().to_string(),
            })?;
            return Ok(true);
        }

        if nombre != r"\publicText" || argumentos.trim().is_empty() {
            if nombre != r"\newStatus" {
                return Ok(false);
            }

            let Ok(status) = argumentos.trim().parse::<Status>() else {
                return Ok(false);
            };

            self.enviar(&Mensaje::Status { status })?;
            return Ok(true);
        }

        self.enviar(&Mensaje::PublicText {
            text: argumentos.trim().to_string(),
        })?;

        Ok(true)
    }

    /// Inicia un hilo que recibe y muestra mensajes del servidor.
    pub fn iniciar_escucha(&self) -> Result<(), Error> {
        let stream = self
            .stream
            .as_ref()
            .ok_or_else(|| Error::other("El cliente no está conectado"))?
            .try_clone()?;

        thread::spawn(move || {
            let mut reader = BufReader::new(stream);
            let mut mensaje = String::new();

            loop {
                match reader.read_line(&mut mensaje) {
                    Ok(0) => {
                        println!("La conexión con el servidor se cerró.");
                        break;
                    }
                    Ok(_) => {}
                    Err(error) => {
                        eprintln!("Error recibiendo mensajes del servidor: {}", error);
                        break;
                    }
                }

                match traductor::deserializa(&mensaje) {
                    Ok(mensaje) => match mensaje {
                        Mensaje::NewUser { username } => {
                            println!("NEW_USER: \"{}\"", username);
                        }
                        Mensaje::Disconnected { username } => {
                            println!("DISCONNECT: \"{}\"", username);
                        }
                        Mensaje::PublicTextFrom { username, text } => {
                            println!("{}: {}", username, text);
                        }
                        Mensaje::PrivateTextFrom { username, text } => {
                            println!("Mensaje privado de {}: {}", username, text);
                        }
                        Mensaje::NewStatus { username, status } => {
                            println!("STATUS: \"{}\" -> {}", username, status);
                        }
                        Mensaje::UserList { users } => {
                            for (username, status) in users {
                                println!("{}: {}", username, status);
                            }
                        }
                        Mensaje::Response { result, extra, .. } => {
                            if let Resultado::NoSuchUser = result {
                                if let Some(username) = extra {
                                    println!("El usuario \"{}\" no existe.", username);
                                } else {
                                    println!("El usuario solicitado no existe.");
                                }
                            } else if let Some(extra) = extra {
                                println!("Respuesta del servidor: {:?} ({})", result, extra);
                            } else {
                                println!("Respuesta del servidor: {:?}", result);
                            }
                        }
                        _ => {}
                    },
                    Err(error) => {
                        eprintln!("Se recibió un mensaje inválido del servidor: {}", error);
                    }
                }
                mensaje.clear();
            }
        });

        Ok(())
    }
}
