#!/usr/bin/env node

const { spawn } = require('child_process');
const path = require('path');

const ext = process.platform === 'win32' ? '.exe' : '';
const binaryPath = path.join(__dirname, `diary-header${ext}`);

const child = spawn(binaryPath, process.argv.slice(2), {
  stdio: 'inherit',
  windowsHide: true
});

child.on('exit', (code) => {
  process.exit(code);
});
