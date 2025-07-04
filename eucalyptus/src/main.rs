use dropbear_engine::WindowConfiguration;

fn main() {
    let config = WindowConfiguration {
        width: 1280.0,
        height: 720.0,
        title: "Eucalyptus, built with dropbear"
    };

    let app = dropbear_engine::App::run(config);
}
