#!/usr/bin/env node

const { execSync } = require('child_process');
const path = require('path');

function getStagedFiles() {
  try {
    const output = execSync('git diff --cached --name-only', { encoding: 'utf8' });
    return output.split('\n').filter(Boolean);
  } catch (e) {
    return [];
  }
}

function hasRustChanges(files) {
  return files.some(f =>
    f.endsWith('.rs') ||
    f === 'src-tauri/Cargo.toml' ||
    f === 'src-tauri/Cargo.lock'
  );
}

function hasFrontendChanges(files) {
  return files.some(f =>
    f.startsWith('src/') ||
    f === 'package.json' ||
    f === 'tsconfig.json' ||
    f === 'vite.config.ts'
  );
}

const files = getStagedFiles();
const errors = [];

console.log('Checking staged files for errors...');

if (hasRustChanges(files)) {
  console.log('Running cargo check for Rust changes...');
  try {
    execSync('cd src-tauri && cargo check', { stdio: 'inherit' });
    console.log('Rust check passed.');
  } catch (e) {
    errors.push('Rust check failed');
  }
}

if (hasFrontendChanges(files)) {
  console.log('Running npm run build for frontend changes...');
  try {
    execSync('npm run build', { stdio: 'inherit' });
    console.log('Frontend build passed.');
  } catch (e) {
    errors.push('Frontend build failed');
  }
}

if (errors.length > 0) {
  console.error('\nPre-commit checks failed:');
  errors.forEach(e => console.error('  -', e));
  process.exit(1);
}

console.log('All pre-commit checks passed!');
