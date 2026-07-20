// Bun web server benchmark
// Run: bun run server.js

Bun.serve({
  port: 3000,
  fetch(req) {
    return new Response(JSON.stringify({ message: "hello" }), {
      headers: { "Content-Type": "application/json" },
    });
  },
});

console.log("Bun server listening on port 3000");
