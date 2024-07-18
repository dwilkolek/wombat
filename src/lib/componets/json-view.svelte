<script lang="ts">
	import { toPng } from 'html-to-image';
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

	let container: HTMLDivElement;
</script>

<div class={`flex`}>
	<div bind:this={container} class="grow">
		<table
			class={`table-auto w-full font-mono font-extralight text-xs ${nested ? '' : 'table-zebra'} text-zinc-400 `}
		>
			<tbody>
				{#each entries as [key, value]}
					<tr>
						<td class={`align-top min-w-28 w-28 ${nested ? 'pl-0' : 'pl-2'} text-right`}
							>{key}:
						</td>
						<td class="text-zinc-300">
							{#if typeof value == 'string'}
								{#if value.includes('\n')}
									<div class="text-slate-400 text-pretty">
										{#each value.split('\n') as line}
											<span class="break-all">
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
											</span>
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
	{#if !nested}
		<div class="flex flex-col min-w-36 w-36">
			<button
				class="m-2 btn btn-active btn-primary btn-xs"
				on:click={async () => {
					await writeText(JSON.stringify(log, null, 2));
				}}>Copy raw json</button
			>
			{#await toPng(container) then dataUrl}
				<div class="border border-primary mx-2 p-2 text-center rounded-lg font-semibold text-xs">
					Right click below to copy as image
					<img class="invert object-fit object-center h-16 w-36" src={dataUrl} />
				</div>
			{/await}
		</div>
	{/if}
</div>
