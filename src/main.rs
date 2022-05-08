use std::{iter, net::TcpListener, sync::Arc, thread, time::Duration};

use actix::{Actor, AsyncContext, StreamHandler};
use actix_web::{
    web::{self, Data, Payload},
    App, Error, HttpRequest, HttpResponse, HttpServer,
};
use actix_web_actors::ws;
use async_stream::stream;
use tokio::sync::watch::{channel, Receiver};

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    // Define communication channel.
    let (sender, receiver) = channel(Message { int: 0 });

    // Spawn sender thread.
    let _handle = thread::spawn(move || loop {
        thread::sleep(Duration::from_secs(2));
        sender
            .send(Message { int: 3 })
            .expect("Failed to write to channel");
    });

    // Define server.
    let listener = TcpListener::bind("127.0.0.1:5568")?;

    let receiver = Data::new(receiver);
    let _server = HttpServer::new(move || {
        App::new()
            .route("/", web::get().to(index))
            .app_data(receiver.clone())
    })
    .listen(listener)?
    .run()
    .await;

    Ok(())
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
    receiver: Arc<Receiver<Message>>,
}

impl Actor for MyWs {
    type Context = ws::WebsocketContext<Self>;

    fn started(&mut self, ctx: &mut Self::Context) {
        let stream = stream! {};

        ctx.add_stream(iter::repeat(async {
            while self.receiver.changed().await.is_ok() {
                yield self.receiver.borrow().to_string()
            }
        }));
    }
}

impl StreamHandler<Result<ws::Message, ws::ProtocolError>> for MyWs {
    fn handle(&mut self, _msg: Result<ws::Message, ws::ProtocolError>, _ctx: &mut Self::Context) {
        println!("Got a message");
    }
}

async fn index(
    req: HttpRequest,
    stream: Payload,
    receiver: Data<Receiver<Message>>,
) -> Result<HttpResponse, Error> {
    ws::start(
        MyWs {
            receiver: receiver.into_inner(),
        },
        &req,
        stream,
    )
}
