<script lang="ts">
	import JsonView from '$lib/componets/json-view.svelte';
	type LogType = { [key: string]: unknown };
	export let log: LogType;
	export let nested: boolean | null | undefined;
	const priorityList = [
		'app',
		'level',
		'timestamp',
		'logger',
		'message',
		'thread',
		'context',
		'mdc'
	];
	$: entries = Object.entries(log);
	$: entries.sort((a, b) => {
		const aPos =
			priorityList.indexOf(a[0]) > -1
				? priorityList.indexOf(a[0])
				: 100 + JSON.stringify(a[1]).length;
		const bPos =
			priorityList.indexOf(b[0]) > -1
				? priorityList.indexOf(b[0])
				: 100 + JSON.stringify(b[1]).length;
		return aPos - bPos;
	});
</script>

<div>
	<table
		class={`table-auto w-full font-mono font-extralight text-xs ${nested ? '' : 'table-zebra'} text-zinc-400 `}
	>
		<tbody>
			{#each entries as [key, value]}
				<tr>
					<td class={`align-top min-w-28 w-28 ${nested ? 'pl-0' : 'pl-2'} text-right`}>{key}: </td>
					<td class="text-zinc-300">
						{#if typeof value == 'string'}
							{#if value.includes('\n')}
								<div class="text-slate-400">
									{#each value.split('\n') as line}
										{@html line
											.replaceAll('<', '&lt;')
											.replaceAll('\t', '&nbsp;&nbsp;&nbsp;&nbsp;')
											.replaceAll(
												/(.*)Caused by(.*)/gi,
												'<span class="text-orange-400">$1Caused by$2</span>'
											)
											.replaceAll(
												/(.*)com.technipfmc(.*)/g,
												'<span class="text-amber-300">$1com.technipfmc$2</span>'
											)}
										<br />
									{/each}
								</div>
							{:else}
								{value}
							{/if}
						{:else}
							<JsonView log={value} nested={true} />
						{/if}
					</td>
				</tr>
			{/each}
		</tbody>
	</table>
</div>
