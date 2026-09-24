use std::env;
use std::error::Error;
use std::net::TcpListener;
use std::thread;

use proyecto1::controlador::manejador::Manejador;

struct Servidor {
    direccion: String,
}

impl Servidor {
    fn new(direccion: &str) -> Self {
        Self {
            direccion: direccion.to_string(),
        }
    }

    fn iniciar(&self) -> Result<TcpListener, Box<dyn Error>> {
        let listener = TcpListener::bind(&self.direccion)?;
        Ok(listener)
    }
}

fn main() {
    let puerto = env::args().nth(1).unwrap_or("1234".to_string());

    let direccion = format!("0.0.0.0:{}", puerto);

    let servidor = Servidor::new(&direccion);
    let listener = servidor.iniciar().expect("No se pudo iniciar el servidor");

    println!("Servidor escuchando en {}", servidor.direccion);

    for stream in listener.incoming() {
        let stream = stream.expect("No se pudo aceptar la conexión");

        thread::spawn(move || {
            let mut manejador = Manejador::nuevo(stream);

            match manejador.verificar() {
                Ok(true) => {
                    Manejador::atender();
                }

                Ok(false) => {}

                Err(e) => {
                    eprintln!("Error: {}", e);
                }
            }
        });
    }
}
