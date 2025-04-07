use postgres::Error as PostgresError;
use postgres::{Client, NoTls};
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use validator::{ValidateEmail};

#[macro_use]
extern crate serde_derive;

//TODO: add password with encryption in user as well
//User Model, Struct with id,username,email

#[derive(Serialize, Deserialize)]
struct User {
    id: Option<i32>,
    username: String,
    email: String,
}

// CONSTANTS

const OK_RESPONSE: &str = "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\n\r\n";
const NOT_FOUND: &str = "HTTP/1.1 404 NOT FOUND\r\n\r\n";
const INTERNAL_SERVER_ERROR: &str = "HTTP/1.1 500 INTERNAL SERVER ERROR\r\n\r\n";

fn main() {
    println!("Server started");

    println!("Initializing the database");

    if let Err(e) = set_database() {
        print!("Error:{}", e);
        return;
    }

    let listener = TcpListener::bind(format!("0.0.0.0:8080")).unwrap();
    println!("server started at port 8080");

    for stream in listener.incoming() {
        match stream {
            Ok(stream) => {
                handle_client(stream);
            }
            Err(e) => {
                println!("Error while listening to the incoming stream {}", e);
            }
        }
    }
}

// INITIAZE DATABASE FUNCTION
fn set_database() -> Result<(), PostgresError> {
    println!("{}", &get_db_url());

    let mut client = Client::connect(&get_db_url(), NoTls)?;

    client.batch_execute(
        "CREATE TABLE IF NOT EXISTS users (
                id SERIAL PRIMARY KEY,
                username VARCHAR NOT NULL,
                email VARCHAR NOT NULL
            )",
    )?;
    Ok(())
}

fn handle_client(mut stream: TcpStream) {
    let mut buffer = [0u8; 1024];
    let mut request = String::new();

    match stream.read(&mut buffer) {
        Ok(size) => {
            request.push_str(String::from_utf8_lossy(&buffer[..size]).as_ref());

            let (status_line, content) = match &*request {
                r if r.starts_with("POST /users") => handle_post_request(r),
                r if r.starts_with("GET /users/") => handle_get_request(r),
                r if r.starts_with("GET /users") => handle_get_all_request(r),
                r if r.starts_with("PUT /users/") => handle_put_request(r),
                r if r.starts_with("DELETE /users/") => handle_delete_request(r),
                _ => (NOT_FOUND.to_string(), "404 Not Found".to_string()),
            };

            stream
                .write_all(format!("{}{}", status_line, content).as_bytes())
                .unwrap();
        }
        Err(e) => {
            println!("Error while reading the stream {}", e);
        }
    }
}

//CONTROLLERS

fn handle_post_request(request: &str) -> (String, String) {
    match (
        get_user_request_body(request),
        Client::connect(&get_db_url(), NoTls),
    ) {
        (Ok(user), Ok(mut client)) => {
            // Validate the user
            if false == user.email.validate_email() {
                return (
                    INTERNAL_SERVER_ERROR.to_string(),
                    "Invalid email format".to_string(),
                );
            }

            client
                .execute(
                    "INSERT INTO users (username, email) VALUES ($1, $2)",
                    &[&user.username, &user.email],
                )
                .unwrap();

            return (OK_RESPONSE.to_string(), "User created".to_string());
        }
        _ => return (INTERNAL_SERVER_ERROR.to_string(), "Error".to_string()),
    }
}

fn handle_get_request(request: &str) -> (String, String) {
    match (
        get_id(&request).parse::<i32>(),
        Client::connect(&get_db_url(), NoTls),
    ) {
        (Ok(id), Ok(mut client)) => {
            match client.query_one("SELECT * FROM users WHERE id = $1", &[&id]) {
                Ok(row) => {
                    let user = User {
                        id: row.get(0),
                        username: row.get(1),
                        email: row.get(2),
                    };
                    return (
                        OK_RESPONSE.to_string(),
                        serde_json::to_string(&user).unwrap(),
                    );
                }
                _ => return (NOT_FOUND.to_string(), "User not found".to_string()),
            }
        }
        _ => return (INTERNAL_SERVER_ERROR.to_string(), "Error".to_string()),
    }
}

fn handle_get_all_request(_request: &str) -> (String, String) {
    match Client::connect(&get_db_url(), NoTls) {
        Ok(mut client) => {
            let mut users = Vec::new();

            for row in client.query("SELECT * FROM users", &[]).unwrap() {
                users.push(User {
                    id: row.get(0),
                    username: row.get(1),
                    email: row.get(2),
                });
            }

            return (
                OK_RESPONSE.to_string(),
                serde_json::to_string(&users).unwrap(),
            );
        }
        _ => return (INTERNAL_SERVER_ERROR.to_string(), "Error".to_string()),
    }
}

fn handle_put_request(request: &str) -> (String, String) {
    match (
        get_id(&request).parse::<i32>(),
        get_user_request_body(&request),
        Client::connect(&get_db_url(), NoTls),
    ) {
        (Ok(id), Ok(user), Ok(mut client)) => {

             // Validate the user
             if false == user.email.validate_email() {
                return (
                    INTERNAL_SERVER_ERROR.to_string(),
                    "Invalid email format".to_string(),
                );
            }
            // Check if user exists
            match client.query_one("SELECT * FROM users WHERE id = $1", &[&id]) {
                Ok(_) => {}
                _ => return (NOT_FOUND.to_string(), "User not found".to_string()),
            }

            client
                .execute(
                    "UPDATE users SET username = $1, email = $2 WHERE id = $3",
                    &[&user.username, &user.email, &id],
                )
                .unwrap();

            return (OK_RESPONSE.to_string(), "User updated".to_string());
        }
        _ => return (INTERNAL_SERVER_ERROR.to_string(), "Error".to_string()),
    }
}

fn handle_delete_request(request: &str) -> (String, String) {
    match (
        get_id(&request).parse::<i32>(),
        Client::connect(&get_db_url(), NoTls),
    ) {
        (Ok(id), Ok(mut client)) => {
            let row_affected = client
                .execute("DELETE FROM users WHERE id = $1", &[&id])
                .unwrap();

            if row_affected == 0 {
                return (NOT_FOUND.to_string(), "User not found".to_string());
            }

            return (OK_RESPONSE.to_string(), "User deleted".to_string());
        }
        _ => return (INTERNAL_SERVER_ERROR.to_string(), "Error".to_string()),
    }
}

//UTILS

fn get_id(request: &str) -> &str {
    request
        .split("/")
        .nth(2)
        .unwrap_or_default()
        .split_whitespace()
        .next()
        .unwrap_or_default()
}

fn get_user_request_body(request: &str) -> Result<User, serde_json::Error> {
    serde_json::from_str(request.split("\r\n\r\n").last().unwrap_or_default())
}

fn get_db_url() -> String {
    return std::env::var("DATABASE_URL").unwrap();
}
