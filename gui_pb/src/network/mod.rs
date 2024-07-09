use crate::App;
use core_pb::messages::GuiToGameServerMessage;

impl App {
    pub fn manage_network(&mut self) {
        if let Some(status) = self.data.network.read() {
            self.data.server_status = status;
            self.data.settings = self.data.server_status.settings.clone();
        }
        if self.data.server_status.settings != self.data.settings {
            self.data
                .network
                .send(GuiToGameServerMessage::Settings(self.data.settings.clone()));
            self.data.server_status.settings = self.data.settings.clone();
        }
    }
}
