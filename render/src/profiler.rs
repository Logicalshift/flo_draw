use std::fmt::{Debug};
use std::time::{Duration, Instant};
use std::hash::{Hash};
use std::collections::{VecDeque, HashMap};

const ROLLING_WINDOW_SIZE: usize = 30;

///
/// Time accumulated for a profiled action
///
struct ActionTime {
    count:  usize,
    time:   Duration,
}

///
/// Used for profiling frame-by-frame actions
///
pub struct RenderProfiler<TAction>
where
    TAction:    Copy + Debug + Hash + Eq,
{
    /// The time that the profiler was created
    start_time: Instant,

    /// The number of frames that have been renderered
    frame_count: usize,

    /// The number of primitives rendered for the frame
    frame_primitive_count: usize,

    /// If a frame is being rendered, when it was started
    frame_start: Option<Instant>,

    /// Time when the last frame was started
    last_frame_start: Option<Instant>,

    /// Time when the last frame finished
    last_frame_finish: Option<Instant>,

    /// The time that the last frame finished
    frame_finish: Option<Instant>,

    /// The times when each action was started
    action_start: HashMap<TAction, Instant>,

    /// The counts and accumulated time for the actions in the current frame
    frame_action_times: HashMap<TAction, ActionTime>,

    /// Monitors the times for the last few frames (start, end times)
    rolling_frame_times: VecDeque<(Instant, Instant)>,
}

impl<TAction> RenderProfiler<TAction> 
where
    TAction:    Copy + Debug + Hash + Eq,
{
    ///
    /// Creates a new render profiler
    ///
    pub fn new() -> RenderProfiler<TAction> {
        RenderProfiler {
            start_time:             Instant::now(),
            frame_count:            0,
            frame_primitive_count:  0,
            frame_start:            None,
            last_frame_start:       None,
            last_frame_finish:      None,
            frame_finish:           None,
            action_start:           HashMap::new(),
            frame_action_times:     HashMap::new(), 
            rolling_frame_times:    VecDeque::new(),
        }
    }

    ///
    /// Called when a new frame is rendered
    ///
    #[inline]
    pub fn start_frame(&mut self) {
        // Set the time that this frame started
        self.last_frame_start   = self.frame_start;
        self.last_frame_finish  = self.frame_finish;
        self.frame_start        = Some(Instant::now());

        // No actions have been run this frame, so reset the actions
        self.frame_action_times.clear();
        self.frame_primitive_count = 0;
    }

    ///
    /// An action has started
    ///
    /// Note that actions cannot be nested, so if the action is already running that time will be discarded. Several
    /// different actions can be considered to be running at the same time, however.
    ///
    #[inline]
    pub fn start_action(&mut self, action: TAction) {
        self.action_start.insert(action, Instant::now());
    }

    ///
    /// An action has finished (it is counted as action and the time accumulated)
    ///
    #[inline]
    pub fn finish_action(&mut self, action: TAction) {
        let now = Instant::now();

        if let Some(action_start_time) = self.action_start.get(&action) {
            // Work out how long the action has taken
            let duration = now.duration_since(*action_start_time);

            // Add to the time for this action
            let time = self.frame_action_times
                .entry(action)
                .or_insert_with(|| ActionTime { count: 0, time: Duration::default() });

            time.count  += 1;
            time.time   += duration;
        }
    }

    ///
    /// Indicate that a number of primitives have been rendered this frame
    ///
    #[inline]
    pub fn count_primitives(&mut self, num_primitives: usize) {
        self.frame_primitive_count += num_primitives;
    }

    ///
    /// Finishes the current frame and moves to the next one
    ///
    #[inline]
    pub fn finish_frame(&mut self) {
        // Store the frame finish time
        self.frame_finish = Some(Instant::now());

        // Increase the frame count
        self.frame_count += 1;

        // Update the rolling frames list
        if let (Some(start), Some(end)) = (&self.frame_start, &self.frame_finish) {
            let start   = *start;
            let end     = *end;

            self.rolling_frame_times.push_back((start, end));
            while self.rolling_frame_times.len() > ROLLING_WINDOW_SIZE {
                self.rolling_frame_times.pop_front();
            }
        }
    }

    ///
    /// Generates a summary for the last frame (called after finish_frame)
    ///
    pub fn summary_string(&self) -> String {
        // Calculate some time values
        let total_time      = self.frame_finish.map(|frame_finish| frame_finish.duration_since(self.start_time)).unwrap_or(Duration::default());
        let total_seconds   = (total_time.as_micros() as f64) / 1_000_000.0;

        let rolling_start   = self.rolling_frame_times.iter().next().map(|(start_time, _end_time)| *start_time);
        let rolling_end     = self.rolling_frame_times.iter().last().map(|(_start_time, end_time)| *end_time);
        let rolling_time    = if let (Some(start), Some(end)) = (rolling_start, rolling_end) { end.duration_since(start) } else { Duration::default() };
        let rolling_fps     = (self.rolling_frame_times.len() as f64) / ((rolling_time.as_micros() as f64) / 1_000_000.0);

        let frame_time      = if let (Some(start), Some(end)) = (self.frame_start, self.frame_finish) { end.duration_since(start) } else { Duration::default() };
        let frame_millis    = (frame_time.as_micros() as f64) / 1_000.0;

        let idle_time       = if let (Some(start), Some(end)) = (self.last_frame_finish, self.frame_start) { end.duration_since(start) } else { Duration::default() };
        let idle_millis     = (idle_time.as_micros() as f64) / 1_000.0;

        // Header indicates the frame number, total time and FPS and frame generation time info
        let header = format!("==== FRAME {} @ {:.2}s === {:.1} fps === {:.2}ms = {:.2}ms idle ===",
            self.frame_count,
            total_seconds,
            rolling_fps,
            frame_millis,
            idle_millis);

        // Number of primitives
        let num_primitives = format!("    {} primitives", self.frame_primitive_count);

        // Action time summary for the frame, sorted by slowest action
        let mut all_actions     = self.frame_action_times.iter().collect::<Vec<_>>();
        all_actions.sort_by_key(|(_act, time)| time.time);
        all_actions.reverse();

        let slowest_time        = all_actions.iter().next().map(|(_, slowest_time)| slowest_time.time).unwrap_or(Duration::default());
        let slowest_micros      = slowest_time.as_micros() as f64;
        let all_actions         = all_actions.into_iter()
            .map(|(action, time)| {
                let micros      = time.time.as_micros() as f64;
                let graph_len   = 16.0*(micros/slowest_micros);
                let graph       = "#".repeat(graph_len as _);
                let action      = format!("{:?}", action);

                format!("   {: <20.20} | {: >8.8}µs | {: >7.7} | {}", action, time.time.as_micros(), time.count, graph)
            })
            .collect::<Vec<_>>();
        let action_times = all_actions.join("\n");

        // Draw a graph of the recent frame times/idle times
        let mut times       = vec![];
        let mut last_time   = self.rolling_frame_times.iter().next().map(|(start, _end)| *start).unwrap();

        for (start, end) in self.rolling_frame_times.iter() {
            // Figure out the idle time (time spent waiting since the last frame) and frame time for this frame
            let idle_time   = start.duration_since(last_time);
            let frame_time  = end.duration_since(*start);

            // Add to the list of times
            times.push((idle_time, frame_time));

            // Store the last time a frame ended
            last_time = *end;
        }

        // Work out the time for the longest frame
        let longest_frame_time      = times.iter().map(|(idle, frame)| *idle + *frame).max().unwrap_or(Duration::default());
        let longest_frame_micros    = (longest_frame_time.as_micros() as f64).max(1.0);

        // Make a graph of all the frames, except the first which will have bad idle time
        let graph = times.into_iter().skip(1).map(|(idle, frame)| {
            let idle    = idle.as_micros() as f64;
            let frame   = frame.as_micros() as f64;
            let idle    = (idle / longest_frame_micros * 10.0) as usize;
            let frame   = (frame / longest_frame_micros * 10.0) as usize;

            let idle    = "|".repeat(idle);
            let frame   = "#".repeat(frame);

            idle + &frame
        }).collect::<Vec<_>>();

        // Flip the graph from horizontal to vertical
        let graph_len   = graph.len();
        let graph       = (0..10).into_iter()
            .map(|row| {
                let mut graph_row = vec![' '; graph_len];

                for column in 0..graph.len() {
                    let ypos = 9-row;
                    if graph[column].len() > ypos {
                        graph_row[column] = graph[column].chars().nth(ypos).unwrap_or(' ');
                    }
                }

                graph_row.into_iter().collect::<String>()
            });
        let graph       = graph.map(|row| format!("    |{}", row)).collect::<Vec<_>>().join("\n");
        let graph_xaxis = format!("    +{}", "-".repeat(graph_len));

        // Stick together into a summary string
        format!("\n\n{}\n\n{}\n\n{}\n\n{}\n{}\n",
            header,
            num_primitives,
            action_times,
            graph,
            graph_xaxis)
    }
}
