use cgp_patterns::contexts::App;
use cgp_patterns::traits::CanLoadConfig;

fn main() {
    let app = App {
        config_path: "Cargo.toml".into(),
    };

    app.load_config().unwrap();
}
