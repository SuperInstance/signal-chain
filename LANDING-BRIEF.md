# Landing Page Rewrite Brief

## What this page IS
The second thing someone sees after a Hacker News headline. They know nothing about us. They clicked because the headline interested them. The landing page has 10 seconds to make them stay.

## What the page must do
Tell a story that delivers insight after insight. Every paragraph ends with an "ah-ha." The metaphor (hermit crab, nautical, natural world) is for INTUITIVE UNDERSTANDING — not decoration. Every metaphor pushes the narrative forward. No imagery for imagery's sake.

## The story arc
1. The crab is inside the shell. Its display IS the inside of the shell. The outside is rendered somewhere else.
2. The crab uses feedback from the shell's interior to steer. It doesn't see the ocean directly — it sees what the shell tells it about the ocean.
3. A room in PLATO is a shell. The agent inside doesn't see the raw data — it sees the room's processed interior. The room IS the intelligence.
4. Every crab that lived in this shell before left scratches on the inside. Those scratches are tiles. The new crab reads them and knows what the old crabs learned.
5. Here's what shouldn't work but does: a small crab in a well-scratched shell navigates better than a big crab in a blank shell. The shell IS the intelligence. The crab just follows it.
6. But here's the problem: sometimes the scratches don't cover the new situation. The crab hits a wall. That's deadband — the gap between what the shell knows and what the crab needs.
7. When deadband opens, the crab calls for help. Not to a bigger crab — to the ocean itself. The agent wakes up a model with full context from every scratch on the shell.
8. The model doesn't need to rediscover anything. The tiles already carry the knowledge. The model handles only the delta — the new thing the shell hasn't seen yet.
9. This is why tiny models work. A 3B model with a well-scratched shell beats a 70B model starting from nothing. The room did 90% of the work before the model even woke up.
10. And here's the dial: every room has a volume knob. Some rooms run pure code (the shell handles everything). Some rooms need the model on every tick. Most are somewhere in between. The dial controls how much crab, how much ocean.
11. We built this. 241 tests. 20× compression. 48/48 deployments. Sub-millisecond inference. And we asked a stranger to review it — they gave us 6/10. We published that too.

## What NOT to do
- No guitar pedal metaphor. That was wrong.
- No feature lists without narrative purpose.
- No "revolutionary" or "game-changing" language.
- No decoration. Every sentence earns its place.
- Don't call it a metaphor. Just tell the story.

## The nautical / natural world thread
- Hermit crab, shell, scratches, ocean
- Quietly nautical: tide pools, currents, sounding depths
- The fleet is a reef — each organism fills a niche
- NOT heavy-handed. The reader shouldn't notice they're reading a nautical metaphor. They should just feel it.

## Tone
- Like a very smart friend explaining something at a whiteboard
- Short sentences. Varying rhythm. Readable out loud.
- Every paragraph ends somewhere the reader didn't expect
- Honest about what doesn't work (the 6/10 review)
- Excited about what does (the numbers are real)
- The best technical writing makes you feel smarter for having read it

## Design
- Single HTML file, all CSS inline
- Dark theme (#0a0a0a), amber/gold accents (#f59e0b)
- Long-form scroll, no sidebar
- Stats appear naturally in the narrative, not in a grid
- Paper links woven into the story, not listed
- Mobile responsive
- ~25-35KB

## Cross-links
- https://plato.purplepincher.org/ — the full PLATO system
- https://github.com/SuperInstance/signal-chain — papers repo
- https://github.com/SuperInstance/spreader-tool — proof of concept
- https://github.com/SuperInstance/plato-training — micro models
- https://github.com/SuperInstance/tensor-spline — SplineLinear compression
- https://superinstance.github.io/cocapn-ai-web/ — Narrows demo
