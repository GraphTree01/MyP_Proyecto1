//! Ejecutable del servidor TCP del chat.

use std::env;
use std::error::Error;
use std::net::TcpListener;
use std::sync::{Arc, Mutex};
use std::thread;

use proyecto1::controlador::estado::{EstadoCompartido, EstadoServidor};
use proyecto1::controlador::manejador::Manejador;

/// Estado principal del servidor y registro compartido de usuarios.
struct Servidor {
    direccion: String,
    estado: EstadoCompartido,
}

impl Servidor {
    /// Crea un servidor sin usuarios conectados.
    fn new(direccion: &str) -> Self {
        Self {
            direccion: direccion.to_string(),
            estado: Arc::new(Mutex::new(EstadoServidor::nuevo())),
        }
    }

    /// Reserva la dirección TCP en la que aceptará conexiones.
    fn iniciar(&self) -> Result<TcpListener, Box<dyn Error>> {
        let listener = TcpListener::bind(&self.direccion)?;
        Ok(listener)
    }
}

fn main() {
    if let Err(error) = ejecutar() {
        eprintln!("Error del servidor: {}", error);
    }
}

/// Ejecuta el ciclo que acepta y delega conexiones a hilos independientes.
fn ejecutar() -> Result<(), Box<dyn Error>> {
    let puerto = env::args().nth(1).unwrap_or_else(|| "1234".to_string());

    let direccion = format!("0.0.0.0:{}", puerto);

    let servidor = Servidor::new(&direccion);
    let listener = servidor.iniciar()?;

    println!("Servidor escuchando en {}", servidor.direccion);

    for stream in listener.incoming() {
        let stream = match stream {
            Ok(stream) => stream,
            Err(error) => {
                eprintln!("No se pudo aceptar una conexión: {}", error);
                continue;
            }
        };
        let estado = Arc::clone(&servidor.estado);

        thread::spawn(move || {
            let mut manejador = Manejador::nuevo(stream, estado);

            match manejador.verificar() {
                Ok(true) => {
                    if let Err(e) = manejador.atender() {
                        eprintln!("Error atendiendo al cliente: {}", e);
                    }
                }

                Ok(false) => {}

                Err(e) => {
                    eprintln!("Error: {}", e);
                }
            }
        });
    }

    Ok(())
}
