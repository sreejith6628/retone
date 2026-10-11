// Fake CLI for tests. Mode = first arg.
//  echo        -> writes stdin back to stdout
//  args        -> prints JSON of argv (after mode)
//  sleep [pidfile] -> spawns a grandchild that sleeps 60s (pid written to pidfile), then sleeps 60s itself
//  fail        -> writes "boom" to stderr, exits 3
const mode = process.argv[2];
if (mode === "args") { process.stdout.write(JSON.stringify(process.argv.slice(3))); }
else if (mode === "echo") { const c = []; process.stdin.on("data", d => c.push(d)); process.stdin.on("end", () => process.stdout.write(Buffer.concat(c))); }
else if (mode === "sleep") { const g = require("child_process").spawn(process.execPath, ["-e", "setTimeout(()=>{},60000)"], { stdio: "ignore" }); if (process.argv[3]) require("fs").writeFileSync(process.argv[3], String(g.pid)); setTimeout(() => {}, 60000); }
else if (mode === "fail") { process.stderr.write("boom"); process.exit(3); }
