use cgp_patterns::contexts::App;
use cgp_patterns::traits::CanLoadConfig;

fn main() {
    let app = App {
        config_path: "config.json".into(),
    };

    app.load_config().unwrap();
}
