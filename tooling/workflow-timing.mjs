import { execFileSync } from 'node:child_process';
import { mkdir, writeFile, appendFile } from 'node:fs/promises';
import path from 'node:path';

const { GITHUB_REPOSITORY: repo, GITHUB_RUN_ID: runId, GITHUB_RUN_ATTEMPT: attempt = '1', RUNNER_TEMP: temp, GITHUB_STEP_SUMMARY: summary } = process.env;
if (!repo || !runId || !temp) throw new Error('GitHub run identity and RUNNER_TEMP are required');
const api = endpoint => JSON.parse(execFileSync('gh', ['api', endpoint], { encoding: 'utf8' }));
const endpoint = `repos/${repo}/actions/runs/${runId}/attempts/${attempt}`;
const run = api(endpoint);
const jobs = [];
for (let page = 1; ; page++) {
  const result = api(`${endpoint}/jobs?per_page=100&page=${page}`);
  jobs.push(...result.jobs);
  if (result.jobs.length < 100) break;
}
const duration = (start, end) => {
  const a = Date.parse(start), b = Date.parse(end);
  return a > 0 && b >= a ? Math.round((b - a) / 1000) : null;
};
const format = value => value === null ? '—' : `${Math.floor(value / 60)}m ${value % 60}s`;
const cell = value => String(value ?? '—').replaceAll('|', '\\|').replace(/[\r\n]/g, ' ');
const phase = name => name.startsWith('Post ') ? 'Cache save / Cleanup' : name.includes(' | ') ? name.split(' | ')[0] : 'Setup / Cleanup';
const rows = jobs.filter(job => job.name !== 'Timing summary').map(job => {
  const steps = job.steps.map(step => ({ name: step.name, phase: phase(step.name), conclusion: step.conclusion, seconds: duration(step.started_at, step.completed_at) }));
  const phases = {};
  for (const step of steps) if (step.seconds !== null) phases[step.phase] = (phases[step.phase] ?? 0) + step.seconds;
  const measured = steps.reduce((sum, step) => sum + (step.seconds ?? 0), 0);
  const seconds = duration(job.started_at, job.completed_at);
  if (seconds !== null) phases['Between steps'] = Math.max(0, seconds - measured);
  const hit = steps.find(step => step.name.startsWith('Cache | Rust hit='))?.name.split('hit=')[1] ?? 'not configured';
  return { name: job.name, conclusion: job.conclusion, startedAt: job.started_at, completedAt: job.completed_at, seconds, rustCacheExactHit: hit, phases, steps };
});
const finished = rows.map(job => Date.parse(job.completedAt)).filter(value => Number.isFinite(value) && value > 0);
const firstStarted = rows.map(job => Date.parse(job.startedAt)).filter(value => Number.isFinite(value) && value > 0);
const report = {
  runUrl: run.html_url, attempt: Number(attempt), commit: run.head_sha,
  // Excludes this reporting job; includes scheduling and dependency waits between business jobs.
  pipelineSeconds: finished.length ? duration(run.run_started_at, new Date(Math.max(...finished)).toISOString()) : null,
  initialQueueSeconds: firstStarted.length ? duration(run.run_started_at, new Date(Math.min(...firstStarted)).toISOString()) : null,
  jobs: rows,
};
let md = `# Workflow timings\n\n[Run](${report.runUrl}) · attempt ${attempt} · commit \`${run.head_sha}\`\n\n`;
md += `Pipeline wall time: **${format(report.pipelineSeconds)}**. Initial queue: ${format(report.initialQueueSeconds)}. Parallel jobs are not added together. Reporting/upload overhead is excluded; GitHub's overall duration includes it.\n\n`;
md += '| Job | Result | Duration | Rust cache exact hit |\n| --- | --- | ---: | --- |\n';
for (const job of rows) md += `| ${cell(job.name)} | ${cell(job.conclusion)} | ${format(job.seconds)} | ${cell(job.rustCacheExactHit)} |\n`;
for (const job of rows) {
  md += `\n## ${cell(job.name)}\n\n| Phase | Duration |\n| --- | ---: |\n`;
  for (const [name, seconds] of Object.entries(job.phases)) md += `| ${name} | ${format(seconds)} |\n`;
  md += '\n<details><summary>Step details</summary>\n\n| Step | Result | Duration |\n| --- | --- | ---: |\n';
  for (const step of job.steps) md += `| ${cell(step.name)} | ${cell(step.conclusion)} | ${format(step.seconds)} |\n`;
  md += '\n</details>\n';
}
const dir = path.join(temp, 'workflow-timing');
await mkdir(dir, { recursive: true });
await writeFile(path.join(dir, 'timings.json'), JSON.stringify(report, null, 2) + '\n');
await writeFile(path.join(dir, 'timings.md'), md);
if (summary) await appendFile(summary, md);
console.log(md);
