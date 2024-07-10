use crate::App;
use core_pb::messages::GuiToGameServerMessage;

impl App {
    pub fn manage_network(&mut self) {
        if let Some(status) = self.network.read() {
            self.server_status = status;
            self.settings = self.server_status.settings.clone();
        }
        if self.server_status.settings != self.settings {
            self.network
                .send(GuiToGameServerMessage::Settings(self.settings.clone()));
            self.server_status.settings = self.settings.clone();
        }
    }
}
