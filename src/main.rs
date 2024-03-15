extern crate semver;

use semver::Version;
use std::io::{self, Read, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::Arc;
use std::thread;
use std::time::{Duration, Instant};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

static CLIENT_THREAD_COUNTER: AtomicUsize = AtomicUsize::new(0);
static SHUTDOWN_FLAG: AtomicBool = AtomicBool::new(false);

static mut IS_RUNNING: bool = true;

fn stop_app() {
    // Accessing the static variable from another function
    unsafe {
        IS_RUNNING = false;
    }
    shutdown_server();
}

fn get_is_running() -> bool {
    // Accessing the static variable from another function
    unsafe {
        let is_run = IS_RUNNING;

        return is_run;
    }
}

fn shutdown_server() {
    SHUTDOWN_FLAG.store(true, Ordering::SeqCst);
}

fn handle_client(mut stream: TcpStream) {
    // Increment the thread counter
    CLIENT_THREAD_COUNTER.fetch_add(1, Ordering::SeqCst);

    let mut isAuthenticated = false;
    let password = "EPICPROPASSWORD6969";

    let mut buffer = [0; 1024];

    match stream.read(&mut buffer) {
        Ok(bytes_read) => {
            if bytes_read == 0 {
                println!("Client disconnected");
            }

            let request = String::from_utf8_lossy(&buffer[..bytes_read]).trim().to_string();

            println!("Password given {}", request);

            if request == password {
                println!("Authenticated user logged in");
                // let response = "1.0.0";
                // stream.write(response.as_bytes()).expect("Failed to write response!");
                isAuthenticated = true;
            } else {
                println!("Non-authenticated user kicked");

                // Close the TCP connection
                //drop(stream);
            }
        }
        Err(err) => {
            eprintln!("Error reading from client (initial): {}", err);
        }
    }

    // Set the stream to non-blocking mode
    if let Err(err) = stream.set_nonblocking(true) {
        eprintln!("Failed to set socket to non-blocking mode: {}", err);
        return; // Exit the function on error
    }

    while get_is_running() && isAuthenticated {
        match stream.read(&mut buffer) {
            Ok(bytes_read) => {
                if bytes_read == 0 {
                    println!("Client disconnected");
                    break;
                }
            }
            Err(err) => {
                if err.kind() == std::io::ErrorKind::WouldBlock {
                    // No data available to read, ignore and continue
                } else {
                    eprintln!("Error reading from client (authenticated): {}", err);
                    break;
                }
            }
        }
    }

    // Decrement the thread counter when the thread finishes
    CLIENT_THREAD_COUNTER.fetch_sub(1, Ordering::SeqCst);

    println!("user thread closed.");
}


fn main() {
    let mut server_version = "1.0.0";
    let mut app_version = "1.0.0";



    let listener = TcpListener::bind("127.0.0.1:8080").expect("Failed to bind to address");
    println!("Server listening on 127.0.0.1:8080");

    println!("Available commands: commands, version, connectedips, appversion, uptime , quit/exit/leave");

    // Spawn a separate thread to handle user input
    thread::spawn(|| {
        let start_time = Instant::now();

        while (get_is_running()) {
            let mut input = String::new();
            io::stdin().read_line(&mut input).expect("Failed to read input");
            let input = input.trim().to_lowercase();

            // Calculate uptime by subtracting start time from current time
            let uptime = start_time.elapsed();

            if input == "commands" {
                println!("Available commands: commands, version, connectedips, appversion, uptime , quit/exit/leave");
            }
            else if input == "uptime" {
                // Print uptime in a human-readable format
                println!("Uptime: {} hours, {} minutes, {} seconds",
                         uptime.as_secs() / 3600,
                         (uptime.as_secs() % 3600) / 60,
                         uptime.as_secs() % 60);
            }
            else if input == "connectedips" {

                let value: usize;


                unsafe {
                    value = CLIENT_THREAD_COUNTER.load(Ordering::SeqCst);
                }

                println!("Connected: {}", value);
            }
            else if input == "quit" || input == "exit" || input == "leave"{
                println!("Shutting down server...");
                stop_app();
            }
            else if input == "version" {
            println!("server version: ");
            match Version::parse(server_version) {
                Ok(version) => {
                    println!("Major: {}", version.major);
                    println!("Minor: {}", version.minor);
                    println!("Patch: {}", version.patch);
                }
                Err(e) => {
                    eprintln!("Error parsing version: {}", e);
                }
            }
        }else if input == "appversion" {
            println!("app version:");
            match Version::parse(app_version) {
                Ok(version) => {
                    println!("Major: {}", version.major);
                    println!("Minor: {}", version.minor);
                    println!("Patch: {}", version.patch);
                }
                Err(e) => {
                    eprintln!("Error parsing version: {}", e);
                }
            }
        }
            else {
                println!("Invalid command. Type commands: commands for help.");
            }
        }
    });

    // Spawn a thread to handle incoming connections
    let listener_thread = thread::spawn(move || {
        // Set a short timeout for non-blocking I/O
        listener.set_nonblocking(true).expect("Failed to set non-blocking mode");

        loop {
            // Check if a shutdown signal has been received
            if SHUTDOWN_FLAG.load(Ordering::SeqCst) {
                break; // Stop listening if the shutdown flag is set
            }

            // Attempt to accept a new client connection
            match listener.accept() {
                Ok((stream, _)) => {
                    // Spawn a new thread to handle each client connection
                    thread::spawn(move || handle_client(stream));
                }
                Err(ref e) if e.kind() == io::ErrorKind::WouldBlock => {
                    // No incoming connection, sleep for a short duration
                    thread::sleep(Duration::from_millis(100));
                }
                Err(e) => {
                    eprintln!("Failed to accept connection: {}", e);
                    break; // Exit the loop on error
                }
            }
        }
    });


    // Wait for the listener thread to finish
    listener_thread.join().expect("Listener thread panicked");
}




// let response = match request.as_str() {
//     "commands available" => "Available commands: Commands available, Show Connected IP's, Quit, UpTime",
//     "show connected ip's" => "List of connected IPs: <your implementation here>",
//     "quit" => {
//         // Optionally, you can implement graceful shutdown logic here
//         return;
//     },
//     "uptime" => {
//         // Optionally, you can implement uptime calculation logic here
//         "Server uptime: <your implementation here>"
//     },
//     _ => "Invalid command",
// };
// stream.write(response.as_bytes()).expect("Failed to write response!");

// fn main() {


//     println!("Command arguments: version, appversion, quit");

//     while is_running {
//         use std::io::{stdin,stdout,Write};
//         let mut s=String::new();
//         print!("command input: ");

//         let _=stdout().flush();
//         stdin().read_line(&mut s).expect("Did not enter a correct string");
//         if let Some('\n')=s.chars().next_back() {
//             s.pop();
//         }
//         if let Some('\r')=s.chars().next_back() {
//             s.pop();
//         }

//         if s.trim().to_lowercase() == "version" {
//             println!("server version: ");
//             match Version::parse(server_version) {
//                 Ok(version) => {
//                     println!("Major: {}", version.major);
//                     println!("Minor: {}", version.minor);
//                     println!("Patch: {}", version.patch);
//                 }
//                 Err(e) => {
//                     eprintln!("Error parsing version: {}", e);
//                 }
//             }
//         }else if s.trim().to_lowercase() == "appversion" {
//             println!("app version:");
//             match Version::parse(app_version) {
//                 Ok(version) => {
//                     println!("Major: {}", version.major);
//                     println!("Minor: {}", version.minor);
//                     println!("Patch: {}", version.patch);
//                 }
//                 Err(e) => {
//                     eprintln!("Error parsing version: {}", e);
//                 }
//             }
//         }else if s.trim().to_lowercase() == "quit" {
//             is_running = false;
//         }
            
//     }
// }

