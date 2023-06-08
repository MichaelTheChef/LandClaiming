use std::sync::Arc;
use std::collections::HashMap;
use feather_core::world::ChunkPosition;
use feather_core::Position;
use feather_server::{
    chunk_logic::ChunkHolder,
    player::Player,
    util::{BlockPosition, Gamemode},
    Game, Plugin,
};

#[derive(Default)]
struct LandClaiming {
    claims: HashMap<ChunkPosition, Claim>,
}

#[derive(Default)]
struct Claim {
    owner: String,
    members: Vec<String>,
}

impl Plugin for LandClaiming {
    fn on_enable(&mut self, game: Arc<Game>) {
        let plugin = Arc::new(self.clone());

        game.server
            .event_manager()
            .on_player_interact_block(
                move |event| {
                    plugin.handle_interact_block(event);
                },
                plugin.clone(),
            )
            .on_player_move(
                move |event| {
                    plugin.handle_player_move(event);
                },
                plugin.clone(),
            );

        game.server
            .command_manager()
            .register_command(Box::new(ClaimCommand::new(plugin.clone())))
            .register_command(Box::new(UnclaimCommand::new(plugin.clone())));
    }
}

impl LandClaiming {
    fn handle_interact_block(&self, event: &PlayerInteractBlockEvent) {
        let player = event.player();

        if player.gamemode() == Gamemode::Creative {
            return;
        }

        let block_pos = event.block().position;

        if let Some(claim) = self.get_claim_at(block_pos) {
            let claim_owner = claim.owner;
            let player_name = player.name();

            if claim_owner != player_name && !claim.members.contains(&player_name) {
                player.send_message(format!(
                    "This land is claimed by {}. You cannot interact with it.",
                    claim_owner
                ));
                event.set_cancelled(true);
            }
        }
    }

    fn handle_player_move(&self, event: &PlayerMoveEvent) {
        let player = event.player();
        let new_pos = event.new_position();

        let new_chunk_pos = ChunkPosition::new(new_pos.x as i32, new_pos.z as i32);
        if self.is_claimed_chunk(new_chunk_pos) {
            let claim = self.get_claim_at(new_pos);
            let claim_owner = claim.map(|c| c.owner);

            if claim_owner != player.name()
                && !claim.map(|c| c.members.contains(&player.name())).unwrap_or_default()
            {
                player.send_message(format!(
                    "You entered land claimed by {}. Please respect their property.",
                    claim_owner.unwrap_or("Unknown")
                ));
            }
        }
    }

    fn get_claim_at(&self, position: Position) -> Option<&Claim> {
        let chunk_pos = ChunkPosition::from_block_position(position);
        self.claims.get(&chunk_pos)
    }

    fn is_claimed_chunk(&self, chunk_pos: ChunkPosition) -> bool {
        self.claims.contains_key(&chunk_pos)
    }

    fn claim_chunk(&mut self, player: &Player, chunk_pos: ChunkPosition) {
        let claim = Claim {
            owner: player.name().to_owned(),
            members: vec![player.name().to_owned()],
        };
        self.claims.insert(chunk_pos, claim);
        player.send_message("Chunk claimed successfully.");
    }

    fn unclaim_chunk(&mut self, player: &Player, chunk_pos: ChunkPosition) {
        if let Some(claim) = self.claims.get_mut(&chunk_pos) {
            if claim.owner == player.name() {
                self.claims.remove(&chunk_pos);
                player.send_message("Chunk unclaimed successfully.");
            } else {
                player.send_message("You do not have permission to unclaim this chunk.");
            }
        } else {
            player.send_message("This chunk is not claimed.");
        }
    }
}

struct ClaimCommand {
    plugin: Arc<LandClaiming>,
}

impl ClaimCommand {
    fn new(plugin: Arc<LandClaiming>) -> Self {
        Self { plugin }
    }
}

impl Command for ClaimCommand {
    fn execute(&self, _ctx: &mut CommandContext, sender: &dyn CommandSender, _args: Vec<String>) {
        if let Some(player) = sender.as_player() {
            let player_chunk_pos = ChunkPosition::new(
                player.position().x.floor() as i32,
                player.position().z.floor() as i32,
            );

            if !self.plugin.is_claimed_chunk(player_chunk_pos) {
                self.plugin.claim_chunk(player, player_chunk_pos);
            } else {
                player.send_message("This chunk is already claimed.");
            }
        }
    }

    fn get_name(&self) -> String {
        String::from("claim")
    }

    fn get_usage(&self) -> String {
        String::from("/claim")
    }

    fn get_help(&self) -> String {
        String::from("Claims the current chunk.")
    }
}

struct UnclaimCommand {
    plugin: Arc<LandClaiming>,
}

impl UnclaimCommand {
    fn new(plugin: Arc<LandClaiming>) -> Self {
        Self { plugin }
    }
}

impl Command for UnclaimCommand {
    fn execute(&self, _ctx: &mut CommandContext, sender: &dyn CommandSender, _args: Vec<String>) {
        if let Some(player) = sender.as_player() {
            let player_chunk_pos = ChunkPosition::new(
                player.position().x.floor() as i32,
                player.position().z.floor() as i32,
            );

            self.plugin.unclaim_chunk(player, player_chunk_pos);
        }
    }

    fn get_name(&self) -> String {
        String::from("unclaim")
    }

    fn get_usage(&self) -> String {
        String::from("/unclaim")
    }

    fn get_help(&self) -> String {
        String::from("Unclaims the current chunk.")
    }
}
