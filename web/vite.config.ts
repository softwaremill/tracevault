import { sveltekit } from '@sveltejs/kit/vite';
import tailwindcss from '@tailwindcss/vite';
import { execSync } from 'node:child_process';
import { defineConfig } from 'vite';

const commitHash = execSync('git rev-parse --short HEAD').toString().trim();

export default defineConfig({
	plugins: [tailwindcss(), sveltekit()],
	define: {
		__BUILD_TIME__: JSON.stringify(new Date().toISOString()),
		__COMMIT_HASH__: JSON.stringify(commitHash)
	}
});
