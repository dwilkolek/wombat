<script lang="ts">
	import { toPng } from 'html-to-image';
	import JsonView from '$lib/componets/json-view.svelte';
	import { writeText } from '@tauri-apps/plugin-clipboard-manager';
	import { readable, type Readable } from 'svelte/store';
	import { format } from 'date-fns';

	interface Props {
		log: Readable<object | undefined>;
		propPrefix?: string | null | undefined;
		createFilter?: (prop: string, value: unknown) => void;
	}

	let { log, propPrefix, createFilter }: Props = $props();

	enum Tag {
		std,
		dots,
		ext
	}
	function getReducerAcc(): {
		lines: { line: string; tag: Tag }[];
		sinceLastSpecial: number;
		seenSpecial: boolean;
	} {
		return { lines: [], sinceLastSpecial: 0, seenSpecial: false };
	}

	function getFullPropPath(key: string) {
		return propPrefix ? propPrefix + '.' + key : key;
	}

	function regeneratePng(): Promise<string> {
		return new Promise<string>((resolve) => {
			setTimeout(async () => {
				if (container) {
					const dataUrl = await toPng(container, {
						filter: (domNode) => {
							return !domNode.classList || !domNode.classList.contains('skip-in-image-render');
						}
					});
					resolve(dataUrl);
				}
			}, 500);
		});
	}
	function objToList(obj: object): { key: string; value: unknown }[] {
		return Object.entries(obj).map(([k, v]) => {
			return { key: k, value: v };
		});
	}
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

	let entries: { key: string; value: unknown }[] = $state([]);

	let container: HTMLDivElement | undefined = $state();
	let pngPromise = $state(regeneratePng());

	let showCompactBtn = $derived(
		!propPrefix && entries.some(({ value }) => typeof value == 'string' && value.includes('\n'))
	);
	let compactStacktrace = $state(true);

	let activeTags = $derived(compactStacktrace ? [Tag.std, Tag.dots] : [Tag.ext, Tag.std]);

	log.subscribe((log) => {
		entries = objToList(log ?? {}).sort((a, b) => {
			const akey = a.key.split('.')[0];
			const bkey = b.key.split('.')[0];
			const aPos =
				priorityList.indexOf(akey) > -1
					? priorityList.indexOf(akey)
					: 100 + JSON.stringify(a.value).length;
			const bPos =
				priorityList.indexOf(bkey) > -1
					? priorityList.indexOf(bkey)
					: 100 + JSON.stringify(b.value).length;
			return aPos - bPos;
		});
		if (!propPrefix) {
			pngPromise = regeneratePng();
		}
	});
</script>

<div class="pr-36">
	{#if !propPrefix}
		<div class="min-w-36 w-36 absolute right-0">
			<div class="flex flex-col gap-2 p-2">
				{#if showCompactBtn}<button
						class="btn btn-active btn-primary btn-xs"
						onclick={() => {
							compactStacktrace = !compactStacktrace;
							pngPromise = regeneratePng();
						}}>{compactStacktrace ? 'Show Full' : 'Show Compact'}</button
					>
				{/if}
				<button
					class="btn btn-active btn-primary btn-xs"
					onclick={async () => {
						await writeText(JSON.stringify(log, null, 2));
					}}>Copy raw json</button
				>

				<div class="border border-primary p-2 text-center rounded-lg font-semibold text-xs">
					{#await pngPromise}
						Loading preview
					{:then dataUrl}
						Right click bellow to copy as image
						<!-- svelte-ignore a11y_missing_attribute -->
						<img class="invert object-fit object-center h-16 w-36" src={dataUrl} />
					{/await}
				</div>
			</div>
		</div>
	{/if}
	<div bind:this={container} class={`${propPrefix ? 'bg-transparent' : 'bg-base-300'} grow`}>
		<table class="table-auto w-full font-mono font-extralight text-xs text-zinc-400">
			<tbody>
				{#each entries as { key, value }, index (key)}
					<tr class={propPrefix ? 'transparent' : index % 2 == 1 ? 'bg-base-200' : 'bg-base-300'}>
						<td class={`align-top min-w-28 w-28 ${propPrefix ? 'pl-0' : 'pl-2'} text-right`}
							>{key}:
						</td>
						<td class="text-zinc-300">
							{#if typeof value == 'string'}
								{#if value.includes('\n')}
									{@const stacktraceLines = value
										.split('\n')
										.map((line) => {
											let formatted = line
												.replaceAll('<s', '&lt;')
												.replaceAll('\t', '&nbsp;&nbsp;&nbsp;&nbsp;');
											let isSpecial = false;
											if (formatted.includes('Caused by')) {
												isSpecial = true;
												formatted = formatted.replaceAll(
													/(.*)Caused by(.*)/gi,
													'<span class="text-orange-400">$1Caused by$2</span>'
												);
											}
											if (formatted.includes('com.technipfmc')) {
												isSpecial = true;
												formatted = formatted.replaceAll(
													/(.*)com.technipfmc(.*)/g,
													'<span class="text-amber-300">$1com.technipfmc$2</span>'
												);
											}

											return { line: formatted, isSpecial };
										})
										.reduce((acc, line, li, arr) => {
											acc.sinceLastSpecial++;
											let tag = Tag.ext;
											const lastLines = [
												acc.lines.at(-1),
												acc.lines.at(-2),
												acc.lines.at(-3)
											].filter((line) => !!line);
											if (acc.seenSpecial == false || line.isSpecial || acc.sinceLastSpecial < 4) {
												tag = Tag.std;
											}
											if (line.isSpecial) {
												acc.seenSpecial = true;
												if (acc.sinceLastSpecial > 1) {
													for (const lastLineIdx in lastLines) {
														const lastLine = lastLines[lastLineIdx];

														if (lastLine && lastLine.tag == Tag.ext) {
															lastLine.tag = Tag.std;
														} else {
															break;
														}
													}
												}

												acc.sinceLastSpecial = 0;
											}
											acc.lines.push({ tag, line: line.line });
											if (arr.length - 1 == li) {
												const newLines = [];
												let extCount = 0;
												for (const line of acc.lines) {
													if (line.tag == Tag.std) {
														if (extCount > 0) {
															newLines.push({
																line: `<span class="text-zinc-700">&nbsp;&nbsp;&nbsp;&nbsp;>>> collapsed ${extCount} lines <<<</span>`,
																tag: Tag.dots
															});
														}
														extCount = 0;
														newLines.push(line);
														continue;
													} else {
														extCount++;
														newLines.push(line);
													}
												}
												acc.lines = newLines;
											}
											return acc;
										}, getReducerAcc()).lines}

									<div class="text-slate-400 text-pretty">
										{#each stacktraceLines as line (line)}
											{#if activeTags.includes(line.tag)}
												<span class="break-all">
													<!-- eslint-disable-next-line -->
													{@html line.line}
												</span>
												<br />
											{/if}
										{/each}
									</div>
								{:else}
									{#if key == 'timestamp'}
										{#if value.match(/[0-9]{4}-[0-9]{2}-[0-9]{2}T[0-9]{2}:[0-9]{2}:[0-9]{2}/)}
											{value} (UTC: {format(value, 'yyyy-MM-dd HH:mm:ss.SSS')}, Local: {format(
												value.at(-1) !== 'Z' ? value + 'Z' : value,
												'yyyy-MM-dd HH:mm:ss.SSS'
											)})
										{:else}
											{value} (Formatted: {format(value, 'yyyy-MM-dd HH:mm:ss.SSS')})
										{/if}
									{:else}
										{value}
									{/if}
									{#if createFilter}
										<button
											class="skip-in-image-render break-all text-gray-600 text-[10px] cursor-pointer hover:text-accent"
											onclick={() => createFilter(getFullPropPath(key), value)}
										>
											+filter
										</button>
									{/if}
								{/if}
							{:else if typeof value == 'object' && value != null}
								{#key value}
									<JsonView
										log={readable(value)}
										propPrefix={getFullPropPath(key)}
										{createFilter}
									/>
								{/key}
							{:else}
								<span class="break-all">{JSON.stringify(value)}</span>
							{/if}
						</td>
					</tr>
				{/each}
			</tbody>
		</table>
	</div>
</div>
