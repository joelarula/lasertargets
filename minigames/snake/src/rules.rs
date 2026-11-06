//! Snake rules without ECS: the grid, the snake and one move. Systems call these and turn the
//! outcomes into events and visuals.

use std::collections::VecDeque;

use bevy::asset::uuid::Uuid;
use bevy::prelude::*;

use crate::types::*;

/// The play field in cells. Positions wrap around its edges.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Grid {
    pub size: IVec2,
}

impl Grid {
    /// As many whole cells as fit in the scene (at least 4×4)
    pub fn fitting(scene_size: Vec2) -> Self {
        Self {
            size: (scene_size / CELL_SIZE)
                .floor()
                .as_ivec2()
                .max(IVec2::splat(4)),
        }
    }

    pub fn center(self) -> IVec2 {
        self.size / 2
    }

    /// Leaving one edge comes back in at the opposite one
    pub fn wrap(self, cell: IVec2) -> IVec2 {
        cell.rem_euclid(self.size)
    }

    /// Cell centre in scene-local metres, with the grid centred on the origin
    pub fn to_local(self, cell: IVec2) -> Vec3 {
        ((cell.as_vec2() + 0.5 - self.size.as_vec2() / 2.0) * CELL_SIZE).extend(0.0)
    }

    /// Whether two neighbouring body cells are on opposite edges (the body wrapped between them)
    pub fn wraps_between(a: IVec2, b: IVec2) -> bool {
        (a - b).abs().max_element() > 1
    }

    /// Every cell at least `margin` cells in from the edges
    fn cells(self, margin: IVec2) -> impl Iterator<Item = IVec2> {
        let (min, max) = (margin, self.size - margin);
        (min.y..max.y).flat_map(move |y| (min.x..max.x).map(move |x| IVec2::new(x, y)))
    }

    /// Gems keep two cells away from the edges on grids big enough for that
    fn gem_margin(self) -> IVec2 {
        IVec2::select(
            self.size.cmpgt(IVec2::splat(4)),
            IVec2::splat(2),
            IVec2::ZERO,
        )
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Gem {
    pub cell: IVec2,
    pub color: Color,
}

/// What one move did
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StepOutcome {
    Moved,
    AteGem,
    /// The head ran into the body; nothing moved
    Died,
}

/// One round of snake, kept on the server
#[derive(Resource, Debug, Clone)]
pub struct SnakeState {
    pub session_id: Uuid,
    pub grid: Grid,
    /// Occupied cells, head first
    pub body: VecDeque<IVec2>,
    pub direction: SnakeDirection,
    /// Turn asked for since the last move; the next move applies it
    pub queued_direction: Option<SnakeDirection>,
    pub head_color: Color,
    /// Grey at the start, then the colour of the last gem eaten
    pub body_color: Color,
    /// `None` only once the snake covers every cell
    pub gem: Option<Gem>,
    pub gems_eaten: u32,
    /// The tail stays put for this many more moves
    pub pending_growth: usize,
}

impl SnakeState {
    /// A short snake in the middle of the grid, heading right, and a gem somewhere free
    pub fn new(session_id: Uuid, grid: Grid) -> Self {
        let head = grid.center();
        let mut state = Self {
            session_id,
            grid,
            body: (0..START_LENGTH)
                .map(|i| grid.wrap(head - IVec2::X * i))
                .collect(),
            direction: SnakeDirection::Right,
            queued_direction: None,
            head_color: Color::WHITE,
            body_color: Color::srgb(0.6, 0.6, 0.6),
            gem: None,
            gems_eaten: 0,
            pending_growth: 0,
        };
        state.gem = state.place_gem();
        state
    }

    /// Starts a new round in the same session and grid
    pub fn restart(&mut self) {
        *self = Self::new(self.session_id, self.grid);
    }

    pub fn head(&self) -> IVec2 {
        self.body[0]
    }

    pub fn length(&self) -> u32 {
        self.body.len() as u32
    }

    /// Queues a turn for the next move. Turning back into the body is refused (returns false).
    pub fn steer(&mut self, direction: SnakeDirection) -> bool {
        if direction == self.direction.opposite() {
            return false;
        }
        self.queued_direction = Some(direction);
        true
    }

    /// Moves the snake one cell: turn, then eat, grow or die
    pub fn step(&mut self) -> StepOutcome {
        if let Some(direction) = self.queued_direction.take() {
            self.direction = direction;
        }
        let head = self.grid.wrap(self.head() + self.direction.delta());
        let ate = self.gem.is_some_and(|gem| gem.cell == head);

        // The tail cell is free to enter only if the tail leaves it during this move
        let tail_leaves = !ate && self.pending_growth == 0;
        let blocking = self.body.len() - usize::from(tail_leaves);
        if self.body.iter().take(blocking).any(|&cell| cell == head) {
            return StepOutcome::Died;
        }

        self.body.push_front(head);
        if let Some(gem) = self.gem.filter(|_| ate) {
            self.gems_eaten += 1;
            self.pending_growth += GROWTH_PER_GEM;
            self.head_color = gem.color;
            self.body_color = gem.color;
            self.gem = self.place_gem();
        }
        if self.pending_growth > 0 {
            self.pending_growth -= 1;
        } else {
            self.body.pop_back();
        }

        if ate {
            StepOutcome::AteGem
        } else {
            StepOutcome::Moved
        }
    }

    /// Seconds between moves: shorter with every gem, down to the top speed
    pub fn move_interval(&self) -> f32 {
        (INITIAL_TICK_INTERVAL - SPEED_UP_PER_GEM * self.gems_eaten as f32).max(MIN_TICK_INTERVAL)
    }

    /// 0 at the starting pace, 1 at top speed
    pub fn speed(&self) -> f32 {
        ((INITIAL_TICK_INTERVAL - self.move_interval())
            / (INITIAL_TICK_INTERVAL - MIN_TICK_INTERVAL))
            .clamp(0.0, 1.0)
    }

    /// A random free cell away from the edges, else any free cell, else none
    fn place_gem(&self) -> Option<Gem> {
        let pick = |margin: IVec2| {
            let free: Vec<IVec2> = self
                .grid
                .cells(margin)
                .filter(|cell| !self.body.contains(cell))
                .collect();
            (!free.is_empty()).then(|| free[rand::random_range(0..free.len())])
        };
        let cell = pick(self.grid.gem_margin()).or_else(|| pick(IVec2::ZERO))?;
        Some(Gem {
            cell,
            color: GEM_COLORS[rand::random_range(0..GEM_COLORS.len())],
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn snake(size: IVec2) -> SnakeState {
        SnakeState::new(Uuid::nil(), Grid { size })
    }

    /// Snake with the gem moved out of the way (or right in front when `gem_ahead`)
    fn snake_with_gem(gem_ahead: bool) -> SnakeState {
        let mut s = snake(IVec2::new(20, 20));
        let cell = if gem_ahead {
            s.head() + IVec2::X
        } else {
            IVec2::new(2, 2)
        };
        s.gem = Some(Gem {
            cell,
            color: GEM_COLORS[0],
        });
        s
    }

    #[test]
    fn starts_in_the_middle_heading_right() {
        let s = snake(IVec2::new(20, 10));
        assert_eq!(
            s.body,
            [IVec2::new(10, 5), IVec2::new(9, 5), IVec2::new(8, 5)]
        );
        assert_eq!(s.direction, SnakeDirection::Right);
        assert!(!s.body.contains(&s.gem.unwrap().cell));
    }

    #[test]
    fn moves_one_cell_and_keeps_its_length() {
        let mut s = snake_with_gem(false);
        let head = s.head();
        assert_eq!(s.step(), StepOutcome::Moved);
        assert_eq!(s.head(), head + IVec2::X);
        assert_eq!(s.length(), START_LENGTH as u32);
    }

    #[test]
    fn wraps_around_the_edges() {
        let grid = Grid {
            size: IVec2::new(5, 4),
        };
        assert_eq!(grid.wrap(IVec2::new(5, 0)), IVec2::new(0, 0));
        assert_eq!(grid.wrap(IVec2::new(-1, -1)), IVec2::new(4, 3));
        assert!(Grid::wraps_between(IVec2::new(4, 0), IVec2::new(0, 0)));
        assert!(!Grid::wraps_between(IVec2::new(1, 0), IVec2::new(0, 0)));
    }

    #[test]
    fn refuses_to_reverse() {
        let mut s = snake_with_gem(false);
        assert!(!s.steer(SnakeDirection::Left));
        assert!(s.steer(SnakeDirection::Up));
        s.step();
        assert_eq!(s.direction, SnakeDirection::Up);
    }

    #[test]
    fn eating_grows_by_two_recolours_and_speeds_up() {
        let mut s = snake_with_gem(true);
        assert_eq!(s.step(), StepOutcome::AteGem);
        s.gem = Some(Gem {
            cell: IVec2::new(2, 2),
            color: GEM_COLORS[1],
        });
        s.step();
        s.step();
        assert_eq!(s.length(), START_LENGTH as u32 + GROWTH_PER_GEM as u32);
        assert_eq!(s.gems_eaten, 1);
        assert_eq!(s.body_color, GEM_COLORS[0]);
        assert!(s.move_interval() < INITIAL_TICK_INTERVAL && s.speed() > 0.0);
    }

    #[test]
    fn running_into_the_body_dies_without_moving() {
        let mut s = snake_with_gem(false);
        s.body = [
            IVec2::new(5, 5),
            IVec2::new(5, 6),
            IVec2::new(6, 6),
            IVec2::new(6, 5),
            IVec2::new(7, 5),
        ]
        .into();
        s.direction = SnakeDirection::Right;
        let before = s.body.clone();
        assert_eq!(s.step(), StepOutcome::Died);
        assert_eq!(s.body, before);
    }

    #[test]
    fn may_follow_its_tail_but_not_while_growing() {
        // A 2×2 loop: the head moves into the cell the tail is leaving
        let mut s = snake_with_gem(false);
        s.body = [
            IVec2::new(5, 5),
            IVec2::new(5, 6),
            IVec2::new(6, 6),
            IVec2::new(6, 5),
        ]
        .into();
        s.direction = SnakeDirection::Right;
        assert_eq!(s.clone().step(), StepOutcome::Moved);

        s.pending_growth = 1;
        assert_eq!(s.step(), StepOutcome::Died);
    }

    #[test]
    fn no_gem_once_the_board_is_full() {
        let mut s = snake(IVec2::new(4, 4));
        s.body = Grid {
            size: IVec2::new(4, 4),
        }
        .cells(IVec2::ZERO)
        .collect();
        assert_eq!(s.place_gem(), None);
        s.body.pop_back();
        assert_eq!(s.place_gem().map(|g| g.cell), Some(IVec2::new(3, 3)));
    }

    #[test]
    fn restart_keeps_session_and_grid() {
        let mut s = snake_with_gem(true);
        s.step();
        s.restart();
        assert_eq!(s.gems_eaten, 0);
        assert_eq!(s.length(), START_LENGTH as u32);
        assert_eq!(s.grid.size, IVec2::new(20, 20));
    }
}
