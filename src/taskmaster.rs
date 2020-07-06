use super::environment::Environment;

struct Taskmaster {
    env: &Environment,
}

impl Taskmaster {
    fn new(env: &Environment) -> Taskmaster {
        Taskmaster {
            env
        }
    }

    fn evaluate_schedules() {
        // Look for any schedules that are supposed to be started
    }

    fn begin_unfinished_schedules() {
    }
}
