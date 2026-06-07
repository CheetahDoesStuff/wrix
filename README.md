# Wrix
**A very simple chat app, made to compare axum and arctix**
![image](https://cdn.hackclub.com/019ea268-2b4b-7a03-89fd-8f40884f544d/swappy-20260607_160619.png)

## Why?
Wrix was not built to be a useful chat app, it was simply made to compare Axum and Actix-web in a practical project. It is not intended to be used in a serious context or for anything important and is just a simple demo to compare 2 frameworks.

## Features
The app features the basic stuff:
- Chatting
- Auth
- Login as guest
- Login using HackClub OAuth2
- Custom usernames
- Blazingly fast (Rust ;D)

## Usage
The app is hosted and availble publicly at https://wrix.ch0.dev/.  
Note: You CAN selfhost, but a lot of stuff is hardcoded in the codebase, making it basically impossible to selfhost without changing the code yourself, and since its not made to be hosted by others i am not going to cover it.

## Tech Stack
- Rust
- Axum
- SQLite (through rusqlite)
- Tailwind CSS & HTML/JS

## License
This project is dual licensed under the MIT and APACHE 2.0 Licenses.