#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ActorSlots {
    RED,
    GREEN,
    BLUE,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Snake {
    uuid: Uuid,
    actor: Uuid,
    path: UniversalPath,
    color: Color,
    lives: u8,
    score: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Snak {
    uuid: Uuid,
    name: String,
    actor: Uuid,
    path: UniversalPath,
    slot: ActorSlots,
    value: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SnakeGame {
    game: Game,
    snakes: Vec<Snake>,   
    snaks: Vec<Snak>,
    points_to_win: u32,
}
