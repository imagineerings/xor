use gpui::{Animation, Context, pulsating_between};
use settings::SettingsStore;
use std::time::{Duration, Instant};
use ui::App;

const SMOOTH_BLINK_FRAME_INTERVAL: Duration = Duration::from_millis(16);

pub struct BlinkManager {
    blink_interval: Duration,
    smooth_animation: Animation,
    smooth_started_at: Instant,
    blink_epoch: usize,
    /// Whether the blinking is paused.
    blinking_paused: bool,
    /// Whether the cursor should be visibly rendered or not.
    visible: bool,
    /// Whether the blinking currently enabled.
    enabled: bool,
    /// Whether the blinking is enabled in the settings.
    blink_enabled_in_settings: fn(&App) -> bool,
    /// Whether the cursor should use smooth opacity instead of phase-based blink.
    smooth_blink_enabled_in_settings: fn(&App) -> bool,
}

impl BlinkManager {
    pub fn new(
        blink_interval: Duration,
        smooth_blink_duration: Duration,
        blink_enabled_in_settings: fn(&App) -> bool,
        smooth_blink_enabled_in_settings: fn(&App) -> bool,
        cx: &mut Context<Self>,
    ) -> Self {
        // Make sure we blink the cursors if the setting is re-enabled
        cx.observe_global::<SettingsStore>(move |this, cx| {
            this.blink_cursors(this.blink_epoch, cx)
        })
        .detach();

        Self {
            blink_interval,
            smooth_animation: Animation::new(smooth_blink_duration)
                .repeat()
                .with_easing(pulsating_between(0.3, 1.0)),
            smooth_started_at: Instant::now(),
            blink_epoch: 0,
            blinking_paused: false,
            visible: true,
            enabled: false,
            blink_enabled_in_settings,
            smooth_blink_enabled_in_settings,
        }
    }

    fn next_blink_epoch(&mut self) -> usize {
        self.blink_epoch += 1;
        self.blink_epoch
    }

    pub fn pause_blinking(&mut self, cx: &mut Context<Self>) {
        self.show_cursor(cx);

        let epoch = self.next_blink_epoch();
        let interval = Duration::from_millis(500);
        cx.spawn(async move |this, cx| {
            cx.background_executor().timer(interval).await;
            this.update(cx, |this, cx| this.resume_cursor_blinking(epoch, cx))
        })
        .detach();
    }

    fn resume_cursor_blinking(&mut self, epoch: usize, cx: &mut Context<Self>) {
        if epoch == self.blink_epoch {
            self.blinking_paused = false;
            self.blink_cursors(epoch, cx);
        }
    }

    fn blink_cursors(&mut self, epoch: usize, cx: &mut Context<Self>) {
        if !(self.blink_enabled_in_settings)(cx) {
            self.show_cursor(cx);
            return;
        }

        if epoch == self.blink_epoch && self.enabled && !self.blinking_paused {
            if (self.smooth_blink_enabled_in_settings)(cx) {
                self.visible = true;
                cx.notify();

                let epoch = self.next_blink_epoch();
                cx.spawn(async move |this, cx| {
                    cx.background_executor()
                        .timer(SMOOTH_BLINK_FRAME_INTERVAL)
                        .await;
                    if let Some(this) = this.upgrade() {
                        this.update(cx, |this, cx| this.blink_cursors(epoch, cx));
                    }
                })
                .detach();
            } else {
                self.visible = !self.visible;
                cx.notify();

                let epoch = self.next_blink_epoch();
                let interval = self.blink_interval;
                cx.spawn(async move |this, cx| {
                    cx.background_executor().timer(interval).await;
                    if let Some(this) = this.upgrade() {
                        this.update(cx, |this, cx| this.blink_cursors(epoch, cx));
                    }
                })
                .detach();
            }
        }
    }

    pub fn show_cursor(&mut self, cx: &mut Context<BlinkManager>) {
        if !self.visible {
            self.visible = true;
            cx.notify();
        }
    }

    /// Enable the blinking of the cursor.
    pub fn enable(&mut self, cx: &mut Context<Self>) {
        if self.enabled {
            return;
        }

        self.enabled = true;
        self.smooth_started_at = Instant::now();
        // Set cursors as invisible and start blinking: this causes phase-based
        // cursors to be visible during the next render.
        self.visible = false;
        self.blink_cursors(self.blink_epoch, cx);
    }

    /// Disable the blinking of the cursor.
    pub fn disable(&mut self, _cx: &mut Context<Self>) {
        self.visible = false;
        self.enabled = false;
    }

    pub fn visible(&self) -> bool {
        self.visible
    }

    pub fn opacity(&self, cx: &App) -> f32 {
        if !self.enabled || self.blinking_paused || !(self.blink_enabled_in_settings)(cx) {
            return 1.0;
        }

        if !(self.smooth_blink_enabled_in_settings)(cx) {
            return if self.visible { 1.0 } else { 0.0 };
        }

        let elapsed = self.smooth_started_at.elapsed().as_secs_f32();
        let duration = self.smooth_animation.duration.as_secs_f32();
        if duration == 0.0 {
            return 1.0;
        }

        let delta = elapsed.rem_euclid(duration) / duration;
        (self.smooth_animation.easing)(delta)
    }
}
