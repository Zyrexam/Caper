//! server (static + HMR + wasm rebuild + self-restart).

mod api;
mod assistant_text;
mod auth;
mod automations;
mod events;
mod hostgit;
mod local_projects;
mod sbox;

use rustex1_devserver::{args_to_config, Config, Server};

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let root = std::env::current_dir().expect("current dir");
    let (config, rebuild) = args_to_config(&args, Config::discover(&root));

    // hand, otherwise leaves a row the picker offers and every mount of
    hostgit::sweep(&api::project_root());

    // automation scheduler below can never win the race to build it with a
    let _ = api::angela(config.port);
    let mut server = Server::new(config)
        .gate(std::sync::Arc::new(auth::GateImpl))
        .raw("/api/", api::handle_api)
        .raw("/media/", api::handle_api)
        .on_start(|state| {
            events::start(state);
            automations::start();
        });
    if rebuild {
        server = server.rebuild_on_start();
    }
    server.run();
}