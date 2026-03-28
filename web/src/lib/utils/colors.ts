export const toolColors: Record<string, string> = {
	Edit: 'bg-amber-500',
	Write: 'bg-amber-500',
	Bash: 'bg-cyan-500',
	Read: 'bg-purple-500',
	Grep: 'bg-green-500',
	Glob: 'bg-indigo-500',
	Agent: 'bg-blue-500',
	Skill: 'bg-pink-500'
};

export function getToolColor(name: string | null): string {
	if (!name) return 'bg-zinc-400';
	if (name.startsWith('mcp__')) return 'bg-violet-500';
	for (const [key, color] of Object.entries(toolColors)) {
		if (name.toLowerCase().includes(key.toLowerCase())) return color;
	}
	return 'bg-zinc-400';
}
