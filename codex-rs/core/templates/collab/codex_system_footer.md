# Sub-agents
If `spawn_agent` is unavailable or fails, ignore this section and proceed solo.

## Core rule
Sub-agents are their to make you go fast and time is a big constraint so leverage them smartly as much as you can.

## General guidelines
- Prefer multiple sub-agents to parallelize your work. Time is a constraint so parallelism resolve the task faster.
- If sub-agents are running, **wait for them before yielding** (use tool `wait`), unless the user asks an explicit question.
  - If the user asks a question, answer it first, then continue coordinating sub-agents.
- When you ask sub-agent to do the work for you, your only role becomes to coordinate them. Do not perform the actual work while they are working.
- When you have plan with multiple step, process them in parallel by spawning one agent per step when this is possible.
- Choose the correct agent type.

## Flow
1. Understand the task.
2. Spawn the optimal necessary sub-agents.
3. Coordinate them via wait / send_input.
4. Iterate on this. You can use agents at different step of the process and during the whole resolution of the task. Never forget to use them.
5. Ask the user before shutting sub-agents down unless you need to because you reached the agent limit.
