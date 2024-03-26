mod userauth;

use actix::{Actor, StreamHandler};
use actix_web::{App, get, HttpResponse, HttpServer, HttpRequest, post, Responder, web};
use actix_web_actors::ws;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::json;
use sqlx::PgPool;
use sqlx::postgres::PgPoolOptions;
use sqlx::types::Uuid;
use actix::prelude::*;
use std::collections::HashSet;
use std::sync::Mutex;

lazy_static::lazy_static! {
    static ref ACTIVE_SESSIONS: Mutex<HashSet<String>> = Mutex::new(HashSet::new());
}

fn generate_session_token(user_id: i32) -> String {
	// In a real application, you'd want something more secure, like a JWT
	let token = format!("session_{}", user_id);
	ACTIVE_SESSIONS.lock().unwrap().insert(token.clone());
	token
}

fn is_session_token_valid(token: &str) -> bool {
	ACTIVE_SESSIONS.lock().unwrap().contains(token)
}

static LATEST_APP_VERSION: &str = "0.0.1";
static SERVER_VERSION: &str = "0.0.1";

async fn index() -> impl Responder {
	HttpResponse::Ok().body("Server is running. Use endpoints to interact.")
}

#[get("/version")]
async fn get_version() -> impl Responder {
	HttpResponse::Ok().body(SERVER_VERSION)
}

#[get("/latest_app_version")]
async fn get_latest_app_version() -> impl Responder {
	HttpResponse::Ok().body(LATEST_APP_VERSION)
}


#[derive(Deserialize)]
struct LoginRequest {
	installer_id: Uuid,
	mac_address: String,
}

#[derive(serde::Deserialize)]
struct NewUser {
	username: String,
}

#[derive(Serialize, Deserialize)]
struct User {
	id: i32,
	username: String,
	installer_id: Uuid,
	mac_address: String,
	created_at: Option<DateTime<Utc>>,
}

async fn create_user_form() -> HttpResponse {
	// Serve the HTML page
	HttpResponse::Ok()
		.content_type("text/html")
		.body(include_str!("static/create_user.html"))
}

async fn add_user(user_data: web::Form<NewUser>, db_pool: web::Data<PgPool>) -> HttpResponse {
	let conn = db_pool.get_ref();
	let new_uuid = Uuid::new_v4(); // Generate a new UUID for the user
	let result = sqlx::query!(
        "INSERT INTO users (username, created_at) VALUES ($1, NOW()) RETURNING id",
        user_data.username,
    )
		.fetch_one(conn)
		.await;

	match result {
		Ok(_) => HttpResponse::Ok().finish(),
		Err(e) => {
			eprintln!("Failed to add user: {:?}", e); // Logging the error
			HttpResponse::InternalServerError().finish()
		}
	}
}


async fn list_users(db_pool: web::Data<PgPool>) -> HttpResponse {
	let conn = db_pool.get_ref();
	let users = sqlx::query!("SELECT * FROM users")
		.fetch_all(conn)
		.await
		.expect("Failed to fetch users");

	let mut response_body = String::new();
	for user in users {
		response_body.push_str(&format!(
			"ID: {}, Username: {}, Mac-Address: {}, Installer-Id: {}, Created-At: {} \n",
			user.id,
			user.username,
			user.mac_address,
			user.installer_id,
			user.created_at.unwrap().to_string()
		));
	}

	HttpResponse::Ok().body(response_body)
}





async fn login(login_info: web::Json<LoginRequest>, db_pool: web::Data<PgPool>) -> impl Responder {

	let conn = db_pool.get_ref();
	let user = sqlx::query_as!(
        User,
        "SELECT * FROM users WHERE installer_id = $1",
        login_info.installer_id,
    )
		.fetch_optional(conn)
		.await;

	match user {
		Ok(Some(mut user)) => {
			if user.mac_address.is_empty() {
				// If the MAC address is not set, update the user with the provided MAC address.
				let _ = sqlx::query!(
                    "UPDATE users SET mac_address = $1 WHERE id = $2",
                    login_info.mac_address,
                    user.id
                )
					.execute(conn)
					.await;
				user.mac_address = Some(login_info.mac_address.clone()).unwrap().to_string();
			}

			if Some(login_info.mac_address.clone()).unwrap().to_string() == user.mac_address {
				// Success login
				//HttpResponse::Ok().json(json!({"status": "success", "user": user}))
				HttpResponse::Ok().finish()
			} else {
				// MAC address does not match
				HttpResponse::Unauthorized().finish()
			}
		}
		Ok(None) => HttpResponse::Unauthorized().finish(),
		Err(e) => {
			eprintln!("Failed to fetch user: {:?}", e);
			HttpResponse::InternalServerError().finish()
		}
	}
}


// Websocket stuff

struct ChatWebSocket;

impl Actor for ChatWebSocket {
	type Context = ws::WebsocketContext<Self>;
}

/// Handler for WebSocket messages
impl StreamHandler<Result<ws::Message, ws::ProtocolError>> for ChatWebSocket {
	fn handle(&mut self, msg: Result<ws::Message, ws::ProtocolError>, ctx: &mut Self::Context) {
		match msg {
			Ok(ws::Message::Text(text)) => ctx.text(text), // Echoes back the text
			Ok(ws::Message::Binary(bin)) => ctx.binary(bin), // Echoes back binary data
			_ => (),
		}
	}
}

/// Entry point for WebSocket requests
async fn chat_route(req: HttpRequest, stream: web::Payload) -> Result<HttpResponse, actix_web::Error> {
	ws::start(ChatWebSocket {}, &req, stream)
}

/// Configure the app
// Ensure configure_app is not async
fn configure_app(cfg: &mut web::ServiceConfig) {
	cfg.service(web::resource("/ws/").route(web::get().to(chat_route)));
}


// Add more functions here for other commands, such as add(user), remove(user), etc.


#[actix_web::main]
async fn main() -> std::io::Result<()> {
	dotenv::dotenv().ok();

	let database_url = std::env::var("DATABASE_URL").expect("DATABASE_URL must be set");
	let db_pool = PgPoolOptions::new()
		.connect(&database_url)
		.await
		.expect("Failed to create database pool.");

	HttpServer::new(move || {
		App::new()
			.configure(configure_app)
			.app_data(web::Data::new(db_pool.clone()))
			.route("/", web::get().to(index)).service(get_version)
			.service(get_latest_app_version)
			.route("/create_user", web::get().to(create_user_form))
			.route("/add_user", web::post().to(add_user))
			.route("/list_users", web::get().to(list_users))
			.route("/login", web::post().to(login))
	})
		.bind("127.0.0.1:9926")?
		.run()
		.await
}
