use std::{
    io::{BufRead, BufReader, Write},
    net::{TcpListener, TcpStream},
    process::{Child, Command, Stdio},
    thread,
    time::Duration,
};

use proyecto1::controlador::{
    protocolo::{Mensaje, Operation, Resultado},
    traductor,
};

fn iniciar_servidor() -> (Child, u16) {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let puerto = listener.local_addr().unwrap().port();
    drop(listener);

    let servidor = Command::new(env!("CARGO_BIN_EXE_servidor"))
        .arg(puerto.to_string())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .unwrap();

    for _ in 0..20 {
        if TcpStream::connect(("127.0.0.1", puerto)).is_ok() {
            return (servidor, puerto);
        }

        thread::sleep(Duration::from_millis(50));
    }

    panic!("El servidor no inició a tiempo");
}

fn enviar_mensaje(puerto: u16, mensaje: &Mensaje) -> Mensaje {
    let mut stream = TcpStream::connect(("127.0.0.1", puerto)).unwrap();

    let mut json = traductor::serializa(mensaje).unwrap();
    json.push('\n');

    stream.write_all(json.as_bytes()).unwrap();

    let mut respuesta = String::new();
    BufReader::new(stream)
        .read_line(&mut respuesta)
        .unwrap();

    traductor::deserializa(&respuesta).unwrap()
}

#[test]
fn identifica_cliente_con_nombre_valido() {
    let (mut servidor, puerto) = iniciar_servidor();

    let respuesta = enviar_mensaje(
        puerto,
        &Mensaje::Identify {
            username: "Kimberly".to_string(),
        },
    );

    match respuesta {
        Mensaje::Response {
            operation,
            result,
            extra,
        } => {
            assert!(matches!(operation, Operation::Identify));
            assert!(matches!(result, Resultado::Success));
            assert_eq!(extra.as_deref(), Some("Kimberly"));
        }
        _ => panic!("Respuesta inesperada"),
    }

    servidor.kill().unwrap();
    servidor.wait().unwrap();
}

#[test]
fn rechaza_cliente_con_nombre_invalido() {
    let (mut servidor, puerto) = iniciar_servidor();

    let respuesta = enviar_mensaje(
        puerto,
        &Mensaje::Identify {
            username: "NombreDemasiadoLargo".to_string(),
        },
    );

    match respuesta {
        Mensaje::Response {
            operation,
            result,
            extra,
        } => {
            assert!(matches!(operation, Operation::Identify));
            assert!(matches!(result, Resultado::NotIdentified));
            assert!(extra.is_none());
        }
        _ => panic!("Respuesta inesperada"),
    }

    servidor.kill().unwrap();
    servidor.wait().unwrap();
}

#[test]
fn rechaza_mensaje_que_no_es_identify() {
    let (mut servidor, puerto) = iniciar_servidor();

    let respuesta = enviar_mensaje(
        puerto,
        &Mensaje::NewUser {
            username: "Kimberly".to_string(),
        },
    );

    match respuesta {
        Mensaje::Response {
            operation,
            result,
            ..
        } => {
            assert!(matches!(operation, Operation::Invalid));
            assert!(matches!(result, Resultado::NotIdentified));
        }
        _ => panic!("Respuesta inesperada"),
    }

    servidor.kill().unwrap();
    servidor.wait().unwrap();
}