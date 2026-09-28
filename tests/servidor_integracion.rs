use std::{
    io::{BufRead, BufReader, Write},
    net::{TcpListener, TcpStream},
    process::{Child, Command, Stdio},
    thread,
    time::Duration,
};

use proyecto1::controlador::{
    protocolo::{Mensaje, Operation, Resultado, Status},
    traductor,
};
use std::collections::HashMap;

fn iniciar_servidor() -> (Child, u16) {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let puerto = listener.local_addr().unwrap().port();
    drop(listener);

    let mut servidor = Command::new(env!("CARGO_BIN_EXE_servidor"))
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

    servidor.kill().unwrap();
    servidor.wait().unwrap();
    panic!("El servidor no inició a tiempo");
}

fn enviar_mensaje(puerto: u16, mensaje: &Mensaje) -> Mensaje {
    let mut stream = TcpStream::connect(("127.0.0.1", puerto)).unwrap();

    let mut json = traductor::serializa(mensaje).unwrap();
    json.push('\n');

    stream.write_all(json.as_bytes()).unwrap();

    let mut respuesta = String::new();
    BufReader::new(stream).read_line(&mut respuesta).unwrap();

    traductor::deserializa(&respuesta).unwrap()
}

fn conectar_cliente_identificado(puerto: u16, nombre: &str) -> TcpStream {
    let mut stream = TcpStream::connect(("127.0.0.1", puerto)).unwrap();
    let mensaje = Mensaje::Identify {
        username: nombre.to_string(),
    };
    let mut json = traductor::serializa(&mensaje).unwrap();
    json.push('\n');
    stream.write_all(json.as_bytes()).unwrap();

    let mut respuesta = String::new();
    BufReader::new(stream.try_clone().unwrap())
        .read_line(&mut respuesta)
        .unwrap();
    assert!(matches!(
        traductor::deserializa(&respuesta).unwrap(),
        Mensaje::Response {
            result: Resultado::Success,
            ..
        }
    ));

    stream
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
fn rechaza_nombre_ya_registrado() {
    let (mut servidor, puerto) = iniciar_servidor();
    let _primer_cliente = conectar_cliente_identificado(puerto, "Kimberly");

    let segunda_respuesta = enviar_mensaje(
        puerto,
        &Mensaje::Identify {
            username: "Kimberly".to_string(),
        },
    );
    assert!(matches!(
        segunda_respuesta,
        Mensaje::Response {
            operation: Operation::Identify,
            result: Resultado::UserAlreadyExist,
            extra: Some(_),
        }
    ));

    servidor.kill().unwrap();
    servidor.wait().unwrap();
}

#[test]
fn difunde_texto_publico_a_los_demas_clientes() {
    let (mut servidor, puerto) = iniciar_servidor();
    let mut emisor = conectar_cliente_identificado(puerto, "Emisor");
    let mut receptor = conectar_cliente_identificado(puerto, "Receptor");

    let mensaje = Mensaje::PublicText {
        text: "Hola a todos".to_string(),
    };
    let mut json = traductor::serializa(&mensaje).unwrap();
    json.push('\n');
    emisor.write_all(json.as_bytes()).unwrap();

    let mut respuesta = String::new();
    BufReader::new(&mut receptor)
        .read_line(&mut respuesta)
        .unwrap();

    assert!(matches!(
        traductor::deserializa(&respuesta).unwrap(),
        Mensaje::PublicTextFrom { username, text }
            if username == "Emisor" && text == "Hola a todos"
    ));

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
            operation, result, ..
        } => {
            assert!(matches!(operation, Operation::Invalid));
            assert!(matches!(result, Resultado::NotIdentified));
        }
        _ => panic!("Respuesta inesperada"),
    }

    servidor.kill().unwrap();
    servidor.wait().unwrap();
}

#[test]
fn difunde_cambio_de_status_a_los_demas_clientes() {
    let (mut servidor, puerto) = iniciar_servidor();
    let mut emisor = conectar_cliente_identificado(puerto, "Emisor");
    let mut receptor = conectar_cliente_identificado(puerto, "Receptor");

    let mut notificacion = String::new();
    BufReader::new(&mut emisor)
        .read_line(&mut notificacion)
        .unwrap();
    assert!(matches!(
        traductor::deserializa(&notificacion).unwrap(),
        Mensaje::NewUser { username } if username == "Receptor"
    ));

    let mensaje = Mensaje::Status {
        status: Status::Away,
    };
    let mut json = traductor::serializa(&mensaje).unwrap();
    json.push('\n');
    receptor.write_all(json.as_bytes()).unwrap();

    let mut respuesta = String::new();
    BufReader::new(&mut emisor)
        .read_line(&mut respuesta)
        .unwrap();

    assert!(matches!(
        traductor::deserializa(&respuesta).unwrap(),
        Mensaje::NewStatus { username, status }
            if username == "Receptor" && matches!(status, Status::Away)
    ));

    servidor.kill().unwrap();
    servidor.wait().unwrap();
}

#[test]
fn envia_texto_privado_solo_al_destinatario() {
    let (mut servidor, puerto) = iniciar_servidor();
    let mut emisor = conectar_cliente_identificado(puerto, "Emisor");
    let mut receptor = conectar_cliente_identificado(puerto, "Receptor");

    let mut notificacion = String::new();
    BufReader::new(&mut emisor)
        .read_line(&mut notificacion)
        .unwrap();

    let mensaje = Mensaje::PrivateText {
        username: "Receptor".to_string(),
        text: "Hola en privado".to_string(),
    };
    let mut json = traductor::serializa(&mensaje).unwrap();
    json.push('\n');
    emisor.write_all(json.as_bytes()).unwrap();

    let mut respuesta = String::new();
    BufReader::new(&mut receptor)
        .read_line(&mut respuesta)
        .unwrap();

    assert!(matches!(
        traductor::deserializa(&respuesta).unwrap(),
        Mensaje::PrivateTextFrom { username, text }
            if username == "Emisor" && text == "Hola en privado"
    ));

    servidor.kill().unwrap();
    servidor.wait().unwrap();
}

#[test]
fn informa_si_no_existe_el_destinatario_privado() {
    let (mut servidor, puerto) = iniciar_servidor();
    let mut emisor = conectar_cliente_identificado(puerto, "Emisor");

    let mensaje = Mensaje::PrivateText {
        username: "Inexistente".to_string(),
        text: "Hola".to_string(),
    };
    let mut json = traductor::serializa(&mensaje).unwrap();
    json.push('\n');
    emisor.write_all(json.as_bytes()).unwrap();

    let mut json_respuesta = String::new();
    BufReader::new(&mut emisor)
        .read_line(&mut json_respuesta)
        .unwrap();
    let respuesta = traductor::deserializa(&json_respuesta).unwrap();

    assert!(matches!(
        respuesta,
        Mensaje::Response {
            operation: Operation::Text,
            result: Resultado::NoSuchUser,
            extra: Some(username),
        } if username == "Inexistente"
    ));

    servidor.kill().unwrap();
    servidor.wait().unwrap();
}

#[test]
fn devuelve_la_lista_de_usuarios_con_sus_estados() {
    let (mut servidor, puerto) = iniciar_servidor();
    let mut emisor = conectar_cliente_identificado(puerto, "Emisor");
    let _receptor = conectar_cliente_identificado(puerto, "Receptor");

    let mut notificacion = String::new();
    BufReader::new(&mut emisor)
        .read_line(&mut notificacion)
        .unwrap();

    let mensaje = Mensaje::Users;
    let mut json = traductor::serializa(&mensaje).unwrap();
    json.push('\n');
    emisor.write_all(json.as_bytes()).unwrap();

    let mut respuesta = String::new();
    BufReader::new(&mut emisor)
        .read_line(&mut respuesta)
        .unwrap();

    let mut esperados = HashMap::new();
    esperados.insert("Emisor".to_string(), Status::Active);
    esperados.insert("Receptor".to_string(), Status::Active);

    assert!(matches!(
        traductor::deserializa(&respuesta).unwrap(),
        Mensaje::UserList { users } if users == esperados
    ));

    servidor.kill().unwrap();
    servidor.wait().unwrap();
}

#[test]
fn crea_un_cuarto_y_agrega_al_creador() {
    let (mut servidor, puerto) = iniciar_servidor();
    let mut cliente = conectar_cliente_identificado(puerto, "Kimberly");

    let mut json = traductor::serializa(&Mensaje::NewRoom {
        roomname: "Sala 1".to_string(),
    })
    .unwrap();
    json.push('\n');
    cliente.write_all(json.as_bytes()).unwrap();

    let mut respuesta = String::new();
    BufReader::new(&mut cliente)
        .read_line(&mut respuesta)
        .unwrap();

    assert!(matches!(
        traductor::deserializa(&respuesta).unwrap(),
        Mensaje::Response {
            operation: Operation::NewRoom,
            result: Resultado::Success,
            extra: Some(roomname),
        } if roomname == "Sala 1"
    ));

    servidor.kill().unwrap();
    servidor.wait().unwrap();
}

#[test]
fn rechaza_un_cuarto_con_nombre_repetido() {
    let (mut servidor, puerto) = iniciar_servidor();
    let mut primer_cliente = conectar_cliente_identificado(puerto, "Kimberly");

    let mut json = traductor::serializa(&Mensaje::NewRoom {
        roomname: "Sala 1".to_string(),
    })
    .unwrap();
    json.push('\n');
    primer_cliente.write_all(json.as_bytes()).unwrap();

    let mut respuesta = String::new();
    BufReader::new(&mut primer_cliente)
        .read_line(&mut respuesta)
        .unwrap();

    let mut segundo_cliente = conectar_cliente_identificado(puerto, "Luis");
    segundo_cliente.write_all(json.as_bytes()).unwrap();

    let mut segunda_respuesta = String::new();
    BufReader::new(&mut segundo_cliente)
        .read_line(&mut segunda_respuesta)
        .unwrap();

    assert!(matches!(
        traductor::deserializa(&segunda_respuesta).unwrap(),
        Mensaje::Response {
            operation: Operation::NewRoom,
            result: Resultado::RoomAlreadyExists,
            extra: Some(roomname),
        } if roomname == "Sala 1"
    ));

    servidor.kill().unwrap();
    servidor.wait().unwrap();
}
