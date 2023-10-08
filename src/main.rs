use mdrc_pacbot_util::{gui, network};

fn main() {
    network::start_network_thread();
    gui::run_gui();
}
