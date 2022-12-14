#[macro_use] extern crate rocket; //can use rocket everywhere in app when declared macro

use rocket::fs::FileServer;
use rocket::response::stream::{EventStream, Event};
use rocket::{State, Shutdown};
use rocket::tokio::sync::broadcast::{channel, Sender, error::RecvError}; 
//tokio is async runtime in Rust
//channel is used to pass around messages between different async tasks
use rocket::form::Form;
use rocket::serde::{Serialize, Deserialize};
use rocket::tokio::select;
use rocket::fs::{relative};

// #[get("/world")] //route attribute = GET request to "world path"
// fn world() -> &'static str {
//     "Hello, world!"
// }

#[derive(Debug, Clone, FromForm, Serialize, Deserialize)]
#[serde(crate = "rocket::serde")]
struct Message {
    #[field(validate = len(..30))]
    pub room: String,
    #[field(validate = len(..20))]
    pub username: String,
    pub message: String,
}

#[get("/events")]
async fn events(queue: &State<Sender<Message>>, mut end: Shutdown) -> EventStream![] { //allows server to send data to client whenever it likes
    let mut rx = queue.subscribe(); //to create a new receiver to listen to event 
    EventStream! {
        loop {
            let msg = select! {
                msg = rx.recv() => match msg {
                    Ok(msg) => msg,
                    Err(RecvError::Closed) => break, //means there's no more senders
                    Err(RecvError::Lagged(_)) => continue,
                },
                _ = &mut end => break,
            };

            yield Event::json(&msg);
        }
    }
}

#[post("/message", data = "<form>")]
fn post(form: Form<Message>, queue: &State<Sender<Message>>) { //POST handler takes 2 arguments: FormData (which will be converted to Message struct) and the server state i.e. sender which can send message (see channel below)
    let _res = queue.send(form.into_inner()); //this will fail if no active subscriber - ignoring this error for now
}

#[launch] //launch attribute = launch instance of the web server
fn rocket() -> _ {
    rocket::build()
    .manage(channel::<Message>(1024).0) //this enables server to handle state
    .mount("/", routes![post, events]) //1st argument "/" is the base URL; 2nd argument takes name of handlers above
    .mount("/", FileServer::from(relative!("static"))) //mount the FE
}
