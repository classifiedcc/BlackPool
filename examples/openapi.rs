use {blackpool::subcommand::server::ApiDoc, utoipa::OpenApi};

fn main() {
    println!("{}", ApiDoc::openapi().to_pretty_json().unwrap());
}
