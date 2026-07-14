to: "config/scheduler.yaml"
skip_exists: true
message: "A Scheduler job configuration was added successfully. Run with `cargo roco scheduler --list`."

---
output: stdout
jobs:
  write_content:
      shell: true
      run: "echo roco >> ./scheduler.txt"
      schedule: run every 1 second
      # schedule: "* * * * * * *"
      output: silent
      tags: ['base', 'infra']

  # run_task:
  #     run: "foo"
  #     schedule: "at 10:00 am"
