use std::{net::TcpListener, thread, time::Duration};

use actix::{Actor, AsyncContext, SpawnHandle, StreamHandler};
use actix_web::{
    web::{self, Data, Payload},
    App, Error, HttpRequest, HttpResponse, HttpServer,
};
use actix_web_actors::ws::{self};

use tokio::sync::watch::{channel, Receiver};

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    // Define communication channel.
    let (sender, receiver) = channel(Message { int: 0 });

    // Spawn sender thread.
    let _handle = thread::spawn(move || loop {
        thread::sleep(Duration::from_secs(2));
        println!("Generated a message");
        sender
            .send(Message { int: 3 })
            .expect("Failed to write to channel");
        println!("Sent a message");
    });

    // Define server.
    let listener = TcpListener::bind("127.0.0.1:5568")?;

    let app_state = Data::new(AppState { receiver });
    let _server = HttpServer::new(move || {
        App::new()
            .route("/", web::get().to(index))
            .app_data(app_state.clone())
    })
    .listen(listener)?
    .run()
    .await;

    Ok(())
}

#[derive(Clone)]
struct AppState {
    receiver: Receiver<Message>,
}

#[derive(Debug)]
pub struct Message {
    pub int: i8,
}

impl ToString for Message {
    fn to_string(&self) -> String {
        self.int.to_string()
    }
}

struct MyWs {
    receiver: Receiver<Message>,
    spawn_handle: Option<SpawnHandle>,
}

impl Actor for MyWs {
    type Context = ws::WebsocketContext<Self>;

    fn started(&mut self, ctx: &mut Self::Context) {
        let mut receiver = self.receiver.clone();
        self.spawn_handle = Some(ctx.add_stream(async_stream::stream! {
            while receiver.changed().await.is_ok() {
                yield receiver.borrow().to_string()
            };
        }));
    }
}

impl StreamHandler<String> for MyWs {
    fn handle(&mut self, msg: String, ctx: &mut Self::Context) {
        ctx.pong(msg.as_bytes());
    }
}

async fn index(
    req: HttpRequest,
    stream: Payload,
    app_state: Data<AppState>,
) -> Result<HttpResponse, Error> {
    ws::start(
        MyWs {
            receiver: app_state.receiver.clone(),
            spawn_handle: None,
        },
        &req,
        stream,
    )
}

impl StreamHandler<Result<ws::Message, ws::ProtocolError>> for MyWs {
    fn handle(&mut self, _msg: Result<ws::Message, ws::ProtocolError>, _ctx: &mut Self::Context) {
        print!("Received a message")
    }
}
