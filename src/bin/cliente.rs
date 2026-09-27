use std::env;
use std::io;
use std::net::Ipv4Addr;

use proyecto1::controlador::cliente::Cliente;

fn main() {
    if let Err(error) = ejecutar() {
        eprintln!("Error del cliente: {}", error);
    }
}

fn ejecutar() -> Result<(), Box<dyn std::error::Error>> {
    let argumentos: Vec<String> = env::args().collect();

    let ip: Ipv4Addr = argumentos
        .get(1)
        .ok_or("Falta la dirección IP del servidor")?
        .parse()
        .map_err(|_| "IP inválida")?;

    let puerto: u16 = argumentos
        .get(2)
        .ok_or("Falta el puerto del servidor")?
        .parse()
        .map_err(|_| "Puerto inválido")?;

    let mut nombre = String::new();

    println!("Introduce tu nombre:");

    io::stdin().read_line(&mut nombre)?;

    let nombre = nombre.trim().to_string();

    let mut cliente = Cliente::nuevo(ip, puerto);

    if let Err(error) = cliente.conectar() {
        eprintln!("No se pudo conectar al servidor: {}", error);
        return Ok(());
    }

    if let Err(error) = cliente.identificar(nombre) {
        eprintln!("No se pudo enviar la identificación: {}", error);
        return Ok(());
    }

    if let Err(error) = cliente.iniciar_escucha() {
        eprintln!("No se pudo iniciar la recepción: {}", error);
        return Ok(());
    }

    loop {
        let mut comando = String::new();
        if let Err(error) = io::stdin().read_line(&mut comando) {
            eprintln!("No se pudo leer el comando: {}", error);
            continue;
        }

        if comando.is_empty() {
            break;
        }

        let comando = comando.trim_end_matches(['\r', '\n']);
        match cliente.procesar_comando(comando) {
            Ok(true) => {}
            Ok(false) => eprintln!("Comando no reconocido"),
            Err(error) => eprintln!("No se pudo enviar el mensaje: {}", error),
        }
    }

    Ok(())
}
