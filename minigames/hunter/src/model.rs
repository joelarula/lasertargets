use uuid::Uuid;

struct Hunter {
    uuid: Uuid,
    actor: Uuid,   
    score: u32,
    hits: u32,
}

pub struct Snake {
    uuid: Uuid,
    actor: Uuid,
    path: UniversalPath,
    color: Color,
    lives: u8,
    score: u32,
}

struct Prey {
    actor: Uuid,
    lives: u8,   
    reward: u32,
}

struct HunterGame {
    game: Uuid,
    controller: Uuid,
    hunters: Vec<Hunter>,   
    prey: Vec<Prey>,  
}


#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Snak {
    uuid: Uuid,
    name: String,
    actor: Uuid,
    path: UniversalPath,
    reward: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SnakeGame {
    snakes: Vec<Snake>,   
    snaks: Vec<Snak>,
}
