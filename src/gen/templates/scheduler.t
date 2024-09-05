to: "config/scheduler1.yaml"
skip_exists: true
message: "A Scheduler job configuration was added successfully. Run with `cargo loco run scheduler --list`."

---
output: stdout
jobs:
  - name: "Run command"
    shell:
      command: "echo loco >> ./scheduler.txt"
    cron: "*/1 * * * * *"

#   - name: "Run command"
#     task: 
#       name: "[TASK_NAME]"
#     cron: "*/5 * * * * *"
