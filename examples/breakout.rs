//! Breakout — a classic brick-breaking game built with rukoh.
//!
//! Place `music.ogg` and `impact.ogg` in `examples/assets/` before running.
//!
//! Controls
//!   Left / Right  (or A / D)          move paddle
//!   Space  (or gamepad A button)      launch ball
//!   R                                 restart after win or game over
//!   Escape                            quit

use rukoh::{
    Colour, Font, GamepadButton, KeyCode, Music, Rect, Rukoh, RukohConfig, Sound, SoundParams,
    Vec2, WindowMode,
};

// ── Layout ────────────────────────────────────────────────────────────────────

const W: f32 = 800.0;
const H: f32 = 600.0;

const COLS: usize = 13;
const ROWS: usize = 8;
const BLOCK_W: f32 = 54.0;
const BLOCK_H: f32 = 18.0;
const GAP_X: f32 = 2.0;
const GAP_Y: f32 = 4.0;
/// Left edge of the first block column, centred in the window.
const GRID_LEFT: f32 = (W - (COLS as f32 * BLOCK_W + (COLS - 1) as f32 * GAP_X)) / 2.0;
const GRID_TOP: f32 = 65.0;
const TOTAL_BLOCKS: usize = COLS * ROWS;

const PADDLE_W: f32 = 100.0;
const PADDLE_H: f32 = 12.0;
const PADDLE_Y: f32 = 555.0;
const PADDLE_SPEED: f32 = 500.0;

const BALL_R: f32 = 8.0;
const BALL_SPEED: f32 = 390.0;
const LIVES_START: u32 = 3;

// ── Colours ───────────────────────────────────────────────────────────────────

const COL_BG: Colour = Colour::new(0.04, 0.04, 0.08, 1.0);
const COL_SEPARATOR: Colour = Colour::new(0.18, 0.18, 0.18, 1.0);
const COL_BLOCK_SHADE: Colour = Colour::new(0.0, 0.0, 0.0, 0.35);
const COL_HUD: Colour = Colour::WHITE;
const COL_HUD_DIM: Colour = Colour::new(0.55, 0.55, 0.55, 1.0);
const COL_OVERLAY: Colour = Colour::new(0.0, 0.0, 0.06, 0.80);

/// Row colours, top to bottom — higher rows award more points.
const ROW_COLOURS: [Colour; ROWS] = [
    Colour::RED,
    Colour::ORANGE,
    Colour::YELLOW,
    Colour::LIME,
    Colour::CYAN,
    Colour::BLUE,
    Colour::MAGENTA,
    Colour::LIGHT_GREY,
];

const ROW_POINTS: [u32; ROWS] = [7, 6, 5, 4, 3, 2, 2, 1];

// ── Types ─────────────────────────────────────────────────────────────────────

#[derive(Clone, Copy, PartialEq)]
enum Phase {
    /// Ball is resting on the paddle waiting for launch.
    Ready,
    Playing,
    /// Brief pause after losing a ball.
    Dead,
    Win,
    GameOver,
}

#[derive(Clone, Copy)]
struct Block {
    rect: Rect,
    colour: Colour,
    points: u32,
    active: bool,
}

struct State {
    blocks: [Block; TOTAL_BLOCKS],
    blocks_remaining: usize,
    paddle_x: f32,
    ball_pos: Vec2,
    ball_vel: Vec2,
    score: u32,
    lives: u32,
    phase: Phase,
    /// Countdown (seconds) during the Dead phase.
    dead_timer: f32,
}

// ── State helpers ─────────────────────────────────────────────────────────────

impl State {
    fn new() -> Self {
        let mut s = Self {
            blocks: build_blocks(),
            blocks_remaining: TOTAL_BLOCKS,
            paddle_x: W * 0.5 - PADDLE_W * 0.5,
            ball_pos: Vec2::ZERO,
            ball_vel: Vec2::ZERO,
            score: 0,
            lives: LIVES_START,
            phase: Phase::Ready,
            dead_timer: 0.0,
        };
        s.snap_ball();
        s
    }

    /// Place the ball just above the centre of the paddle.
    fn snap_ball(&mut self) {
        self.ball_pos = Vec2::new(self.paddle_x + PADDLE_W * 0.5, PADDLE_Y - BALL_R - 1.0);
    }

    /// Fire the ball upward at a slight angle.
    fn launch(&mut self) {
        let a = 20.0f32.to_radians();
        self.ball_vel = Vec2::new(a.sin() * BALL_SPEED, -a.cos() * BALL_SPEED);
        self.phase = Phase::Playing;
    }

    fn lose_ball(&mut self) {
        self.lives = self.lives.saturating_sub(1);
        self.dead_timer = 1.5;
        self.phase = Phase::Dead;
    }
}

fn build_blocks() -> [Block; TOTAL_BLOCKS] {
    let zero = Block {
        rect: Rect::new(0.0, 0.0, 0.0, 0.0),
        colour: Colour::WHITE,
        points: 0,
        active: false,
    };
    let mut out = [zero; TOTAL_BLOCKS];
    for row in 0..ROWS {
        for col in 0..COLS {
            let x = GRID_LEFT + col as f32 * (BLOCK_W + GAP_X);
            let y = GRID_TOP + row as f32 * (BLOCK_H + GAP_Y);
            out[row * COLS + col] = Block {
                rect: Rect::new(x, y, BLOCK_W, BLOCK_H),
                colour: ROW_COLOURS[row],
                points: ROW_POINTS[row],
                active: true,
            };
        }
    }
    out
}

// ── Update ────────────────────────────────────────────────────────────────────

/// Advance one frame. Returns the number of blocks broken (used to trigger sound).
fn update(state: &mut State, frame: &rukoh::Frame<'_>, dt: f32) -> u32 {
    let gp = frame.gamepad();

    match state.phase {
        Phase::Win | Phase::GameOver => return 0,

        Phase::Dead => {
            state.dead_timer -= dt;
            if state.dead_timer <= 0.0 {
                if state.lives == 0 {
                    state.phase = Phase::GameOver;
                } else {
                    state.phase = Phase::Ready;
                    state.snap_ball();
                }
            }
            // Allow paddle movement during the death pause so the player can
            // position before the next serve.
        }

        Phase::Ready | Phase::Playing => {}
    }

    // ── Paddle movement ───────────────────────────────────────────────────────

    let mut dir = 0.0f32;
    if frame.is_key_down(KeyCode::Left) || frame.is_key_down(KeyCode::A) {
        dir -= 1.0;
    }
    if frame.is_key_down(KeyCode::Right) || frame.is_key_down(KeyCode::D) {
        dir += 1.0;
    }
    if let Some(gp) = gp {
        let sx = gp.left_stick().x;
        if sx.abs() > 0.1 {
            dir += sx;
        }
        if gp.is_button_down(GamepadButton::DpadLeft) {
            dir -= 1.0;
        }
        if gp.is_button_down(GamepadButton::DpadRight) {
            dir += 1.0;
        }
    }
    state.paddle_x =
        (state.paddle_x + dir.clamp(-1.0, 1.0) * PADDLE_SPEED * dt).clamp(0.0, W - PADDLE_W);

    if matches!(state.phase, Phase::Dead) {
        return 0;
    }

    // ── Ready: ball tracks paddle until launched ───────────────────────────────

    if state.phase == Phase::Ready {
        state.snap_ball();
        let launch = frame.is_key_pressed(KeyCode::Space)
            || gp
                .map(|g| g.is_button_pressed(GamepadButton::South))
                .unwrap_or(false);
        if launch {
            state.launch();
        }
        return 0;
    }

    // ── Ball movement ─────────────────────────────────────────────────────────

    state.ball_pos.x += state.ball_vel.x * dt;
    state.ball_pos.y += state.ball_vel.y * dt;

    // Side walls.
    if state.ball_pos.x - BALL_R < 0.0 {
        state.ball_pos.x = BALL_R;
        state.ball_vel.x = state.ball_vel.x.abs();
    } else if state.ball_pos.x + BALL_R > W {
        state.ball_pos.x = W - BALL_R;
        state.ball_vel.x = -state.ball_vel.x.abs();
    }

    // Top wall.
    if state.ball_pos.y - BALL_R < 0.0 {
        state.ball_pos.y = BALL_R;
        state.ball_vel.y = state.ball_vel.y.abs();
    }

    // Bottom — lose a ball.
    if state.ball_pos.y - BALL_R > H {
        state.lose_ball();
        return 0;
    }

    // ── Paddle collision ──────────────────────────────────────────────────────

    let paddle_rect = Rect::new(state.paddle_x, PADDLE_Y, PADDLE_W, PADDLE_H);
    if state.ball_vel.y > 0.0 && circle_hits_rect(state.ball_pos, BALL_R, paddle_rect) {
        // Reflect angle depends on where the ball hits: centre = straight up, edges = ±60°.
        let t = ((state.ball_pos.x - (state.paddle_x + PADDLE_W * 0.5)) / (PADDLE_W * 0.5))
            .clamp(-1.0, 1.0);
        let a = t * std::f32::consts::FRAC_PI_3;
        state.ball_vel.x = a.sin() * BALL_SPEED;
        state.ball_vel.y = -(a.cos().abs()) * BALL_SPEED;
        // Push the ball clear of the paddle surface.
        state.ball_pos.y = PADDLE_Y - BALL_R - 0.5;
    }

    // ── Block collisions ──────────────────────────────────────────────────────

    let mut broken = 0u32;
    for block in state.blocks.iter_mut() {
        if !block.active {
            continue;
        }
        if let Some((rx, ry)) = ball_vs_block(state.ball_pos, BALL_R, block.rect) {
            block.active = false;
            state.blocks_remaining -= 1;
            state.score += block.points;
            broken += 1;
            if rx {
                state.ball_vel.x = -state.ball_vel.x;
            }
            if ry {
                state.ball_vel.y = -state.ball_vel.y;
            }
            // One block per frame — prevents cascading reflections from corner hits.
            break;
        }
    }

    if state.blocks_remaining == 0 {
        state.phase = Phase::Win;
    }

    broken
}

// ── Collision helpers ─────────────────────────────────────────────────────────

fn circle_hits_rect(pos: Vec2, r: f32, rect: Rect) -> bool {
    let nx = pos.x.clamp(rect.x, rect.x + rect.w);
    let ny = pos.y.clamp(rect.y, rect.y + rect.h);
    let dx = pos.x - nx;
    let dy = pos.y - ny;
    dx * dx + dy * dy < r * r
}

/// Returns `Some((reflect_x, reflect_y))` when the ball overlaps the block.
///
/// Uses the minimum penetration axis to determine which face was hit: if the
/// ball is shallower in X than in Y it came through a left/right face, otherwise
/// through the top/bottom face.
fn ball_vs_block(pos: Vec2, r: f32, rect: Rect) -> Option<(bool, bool)> {
    if !circle_hits_rect(pos, r, rect) {
        return None;
    }
    let pen_left = pos.x - (rect.x - r);
    let pen_right = (rect.x + rect.w + r) - pos.x;
    let pen_top = pos.y - (rect.y - r);
    let pen_bottom = (rect.y + rect.h + r) - pos.y;

    let min_x = pen_left.min(pen_right);
    let min_y = pen_top.min(pen_bottom);

    if min_x < min_y {
        Some((true, false)) // left or right face
    } else {
        Some((false, true)) // top or bottom face
    }
}

// ── Draw ──────────────────────────────────────────────────────────────────────

fn draw(state: &State, frame: &mut rukoh::Frame<'_>, font: &Font, font_lg: &Font) {
    frame.clear(COL_BG);

    // Blocks.
    for block in state.blocks.iter() {
        if !block.active {
            continue;
        }
        frame.draw_rect(block.rect, block.colour);
        // Thin dark shade on each block for definition.
        frame.draw_rect_lines(block.rect, 1.0, COL_BLOCK_SHADE);
    }

    // Paddle.
    frame.draw_rect(
        Rect::new(state.paddle_x, PADDLE_Y, PADDLE_W, PADDLE_H),
        Colour::WHITE,
    );

    // Ball — hidden during the death pause.
    if !matches!(state.phase, Phase::Dead) {
        frame.draw_circle(state.ball_pos, BALL_R, Colour::WHITE);
    }

    // ── HUD ───────────────────────────────────────────────────────────────────

    let score_str = format!("Score  {:>5}", state.score);
    frame.draw_text(font, &score_str, Vec2::new(12.0, 12.0), COL_HUD);

    // Lives as small filled circles in the top-right.
    for i in 0..state.lives {
        frame.draw_circle(
            Vec2::new(W - 14.0 - i as f32 * 22.0, 18.0),
            7.0,
            Colour::WHITE,
        );
    }

    // Separator line below HUD.
    frame.draw_rect(Rect::new(0.0, 42.0, W, 1.0), COL_SEPARATOR);

    // ── Phase-specific UI ─────────────────────────────────────────────────────

    match state.phase {
        Phase::Ready => {
            frame.draw_text(
                font,
                "Space to launch",
                Vec2::new(323.0, H - 22.0),
                COL_HUD_DIM,
            );
        }
        Phase::Dead => {
            frame.draw_text(font, "Ball lost!", Vec2::new(355.0, H * 0.5), COL_HUD_DIM);
        }
        Phase::Win => {
            draw_overlay(
                frame,
                font,
                font_lg,
                "You Win!",
                &format!("Score  {}", state.score),
            );
        }
        Phase::GameOver => {
            draw_overlay(
                frame,
                font,
                font_lg,
                "Game Over",
                &format!("Score  {}", state.score),
            );
        }
        Phase::Playing => {}
    }
}

fn draw_overlay(
    frame: &mut rukoh::Frame<'_>,
    font: &Font,
    font_lg: &Font,
    title: &str,
    subtitle: &str,
) {
    frame.draw_rect(Rect::new(0.0, 0.0, W, H), COL_OVERLAY);
    frame.draw_text(font_lg, title, Vec2::new(305.0, 226.0), Colour::WHITE);
    frame.draw_text(font, subtitle, Vec2::new(336.0, 288.0), COL_HUD_DIM);
    frame.draw_text(
        font,
        "R      play again",
        Vec2::new(336.0, 318.0),
        COL_HUD_DIM,
    );
    frame.draw_text(font, "Escape  quit", Vec2::new(336.0, 342.0), COL_HUD_DIM);
}

// ── Entry point ───────────────────────────────────────────────────────────────

fn main() -> Result<(), rukoh::Error> {
    let mut app = Rukoh::new(RukohConfig {
        title: "Breakout — rukoh",
        window_mode: WindowMode::Windowed {
            width: 800,
            height: 600,
        },
        vsync: true,
        ..Default::default()
    })?;

    let font_bytes = include_bytes!("assets/lexend.ttf");
    let font = Font::load(&app, font_bytes, 18.0)?;
    let font_lg = Font::load(&app, font_bytes, 42.0)?;

    // Audio.
    let impact_bytes =
        std::fs::read("examples/assets/impact.ogg").expect("Place impact.ogg in examples/assets/");
    let music_bytes =
        std::fs::read("examples/assets/music.ogg").expect("Place music.ogg in examples/assets/");
    let impact = Sound::load(&app, &impact_bytes)?;
    let music = Music::load(&app, &music_bytes)?;

    let mut state = State::new();
    let mut music_started = false;

    while let Some(mut frame) = app.next_frame() {
        if frame.is_key_pressed(KeyCode::Escape) {
            break;
        }

        // Start music on the first frame.
        if !music_started {
            frame.play_music(&music);
            music_started = true;
        }

        // Cap dt so a frame-rate spike can't tunnel the ball through geometry.
        let dt = frame.delta_time().min(0.05);

        // Restart on R after win or game over.
        if matches!(state.phase, Phase::Win | Phase::GameOver) && frame.is_key_pressed(KeyCode::R) {
            state = State::new();
        }

        let blocks_broken = update(&mut state, &frame, dt);

        // Play the impact sound for each block destroyed this frame.
        for _ in 0..blocks_broken {
            frame.play_sound(&impact, SoundParams::default());
        }

        draw(&state, &mut frame, &font, &font_lg);
    }

    Ok(())
}
