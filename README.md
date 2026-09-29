# Proyecto1

Un chat grupal con diferentes funcionalidades.

## Requisitos

- Rust y Cargo.
- Docker, opcionalmente, para ejecutar el proyecto dentro de un contenedor.

## Ejecutar con Cargo

Primero inicia el servidor en una terminal:

```bash
cargo run --bin servidor -- 1234
```

El puerto es opcional. Si no se indica, el servidor usa el puerto `1234`:

```bash
cargo run --bin servidor
```

Después inicia uno o más clientes en otras terminales:

```bash
cargo run --bin cliente -- 127.0.0.1 1234
```
Si se ejecuta el servidor en una computadora y un cliente en otra bajo una red local, la dirección IP tiene que ser la que ejecuta al servidor. Esta se da automáticamente por el proveedor de internet. Aquí tanto IP y puerto son campos necesarios al ejecutar un cliente.

El cliente solicitará un nombre. Cada nombre debe ser único entre los clientes conectados y tener máximo 8 caracteres.

## Comandos del cliente

### Enviar texto público

```text
\publicText Hola a todos
```

El salto de línea termina el comando. El mensaje se envía a los demás clientes conectados.

Las líneas vacías se ignoran y los comandos desconocidos se informan en la terminal.

### Cambiar estado

```text
\newStatus ACTIVE
\newStatus AWAY
\newStatus BUSY
```

Todos los usuarios comienzan con estado `ACTIVE`. Al cambiarlo, los demás clientes reciben:

```text
STATUS: "nombre" -> AWAY
```

### Enviar texto privado

```text
\privateText --to "nombre" Hola, este mensaje es privado
```

El servidor envía el mensaje únicamente al usuario indicado. Si el destinatario no está conectado, el emisor recibe una respuesta indicando que no se encontró.

### Desconectarse

```text
\disconnect
```

El cliente solicita una desconexión ordenada y los demás usuarios reciben `DISCONNECTED`.

### Consultar usuarios

```text
\users
```

El servidor responde únicamente al cliente que hizo la solicitud con la lista de usuarios conectados y sus estados.

### Invitar usuarios a un cuarto

```text
\invite --room "Sala 1" --to Luis,Antonio,Fernando
```

Solo un miembro del cuarto puede invitar. La sala y todos los usuarios indicados deben existir. Las invitaciones repetidas y los usuarios que ya son miembros se ignoran.

### Unirse a un cuarto

```text
\joinRoom "Sala 1"
```

El usuario debe tener una invitación pendiente.

### Consultar usuarios de un cuarto

```text
\roomUsers "Sala 1"
```

Solo los miembros que ya se unieron al cuarto pueden consultar sus usuarios y estados.

### Enviar texto a un cuarto

```text
\roomText --room "Sala 1" Hola sala 1
```

Solo los miembros que ya se unieron al cuarto reciben el mensaje. El emisor no recibe una copia.

### Crear un cuarto

```text
\newRoom Sala 1
```

El nombre del cuarto puede tener como máximo 16 caracteres. El usuario que lo crea se incorpora automáticamente como su primer miembro.

## Mensajes visibles

Cuando se conecta un usuario, los demás clientes ven:

```text
NEW_USER: "nombre"
```

Cuando se desconecta un usuario:

```text
DISCONNECT: "nombre"
```

Cuando llega un mensaje público:

```text
nombre: Hola a todos
```

Cuando cambia el estado de un usuario:

```text
STATUS: "nombre" -> ACTIVE/AWAY/BUSY
```

Cuando llega un mensaje privado:

```text
Mensaje privado de "nombre": Hola, este mensaje es privado
```

El protocolo interno utiliza JSON entre el cliente y el servidor, pero el cliente muestra estos mensajes en un formato legible.

## Ejecutar las pruebas

```bash
cargo test
```

Para comprobar el formato del código:

```bash
cargo fmt -- --check
```

## Ejecutar con Docker

Construye la imagen:

```bash
docker build -t proyecto1 .
```

Inicia el servidor dentro del contenedor:

```bash
docker run --rm -p 1234:1234 proyecto1
```

Para indicar otro puerto, por ejemplo `4321`:

```bash
docker run --rm -p 4321:4321 proyecto1 ./servidor 4321
```
Después ejecuta los clientes con Cargo desde otras terminales:

```bash
cargo run --bin cliente -- 127.0.0.1 1234
```
