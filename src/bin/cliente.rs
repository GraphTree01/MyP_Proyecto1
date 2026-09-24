use std::env;
use std::io;
use std::net::Ipv4Addr;

use proyecto1::controlador::cliente::Cliente;

fn main() {
    let argumentos: Vec<String> = env::args().collect();

    let ip: Ipv4Addr = argumentos[1].parse().expect("IP inválida");

    let puerto: u16 = argumentos[2].parse().expect("Puerto inválida");

    let mut nombre = String::new();

    println!("Introduce tu nombre:");

    io::stdin()
        .read_line(&mut nombre)
        .expect("No se pudo leer el nombre");

    let nombre = nombre.trim().to_string();

    let mut cliente = Cliente::nuevo(ip, puerto);

    cliente.conectar().expect("No se pudo conectar al servidor");

    cliente.identificar(nombre).expect("No se pudo identificar");
}
