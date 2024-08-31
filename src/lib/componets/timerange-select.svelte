<script lang="ts">
	import type { Timerange, TimeUnit } from '$lib/types';
	import { endOfDay, format, startOfDay } from 'date-fns';
	import { get, type Writable } from 'svelte/store';

	const withZeros = (v: number) => {
		return v < 10 ? `0${v}` : v;
	};
	const toLocalDateStr = (date: Date) => {
		return `${date.getUTCFullYear()}-${withZeros(date.getMonth() + 1)}-${withZeros(
			date.getDate()
		)}T${withZeros(date.getHours())}:${withZeros(date.getMinutes())}:${withZeros(
			date.getSeconds()
		)}`;
	};

	const formatTimerange = (time: Timerange): string => {
		switch (time.type) {
			case 'absolute':
				return format(time.from, 'yyyy-MM-dd HH:mm') + ' → ' + format(time.to, 'yyyy-MM-dd HH:mm');
			case 'relative':
				return (
					'last ' +
					time.amount +
					' ' +
					(time.amount <= 1 ? time.unit.substring(0, time.unit.length - 1) : time.unit)
				);
		}
	};

	export let range: Writable<Timerange>;
	let type = 'relative';
	let from = startOfDay(new Date());
	let to = endOfDay(new Date());
	let amount = 30;
	let unit: TimeUnit = 'minutes';

	range.subscribe((r) => {
		type = r.type;
		switch (r.type) {
			case 'absolute':
				from = r.from;
				to = r.to;
				break;
			case 'relative':
				amount = r.amount;
				unit = r.unit;
				break;
		}
	});
	let details: HTMLDetailsElement;
	let open = false;

	function setTimeUnit(value: string) {
		unit = value as TimeUnit;
	}

	function reset() {
		const storedRange = get(range);
		type = storedRange.type;
		from = startOfDay(new Date());
		to = endOfDay(new Date());
		amount = 30;
		unit = 'minutes';
		switch (storedRange.type) {
			case 'absolute':
				from = storedRange.from;
				to = storedRange.to;
				break;
			case 'relative':
				amount = storedRange.amount;
				unit = storedRange.unit;
				break;
		}
	}
</script>

<details class="dropdown grow" bind:this={details}>
	<summary
		class="btn btn-sm w-[450px]"
		on:click={() => {
			open = true;
		}}
	>
		{formatTimerange($range)}
	</summary>
	<div class="menu dropdown-content bg-base-300 rounded-box z-[1] p-2 shadow w-[450px] -my-8">
		<div class="flex flex-col gap-2">
			<div class="flex gap-2">
				<button
					class={`btn btn-sm ${type == 'relative' ? 'btn-primary' : 'btn-ghost'}`}
					on:click={() => {
						type = 'relative';
					}}>Relative</button
				>
				<button
					class={`btn btn-sm ${type == 'absolute' ? 'btn-primary' : 'btn-ghost'}`}
					on:click={() => {
						type = 'absolute';
					}}>Absolute</button
				>
			</div>
			{#if type == 'relative'}
				<div class="flex gap-2">
					<div class="grow">
						Amount: <input
							type="number"
							class="input input-sm input-bordered w-full"
							on:change={(event) => {
								amount = parseInt(event.currentTarget.value);
							}}
							value={amount}
						/>
					</div>
					<div class="grow">
						Unit: <select
							class="input input-sm input-bordered w-full"
							on:change={(event) => setTimeUnit(event.currentTarget.value)}
						>
							<option value="minutes" selected={unit == 'minutes'}>Minute</option>
							<option value="hours" selected={unit == 'hours'}>Hour</option>
						</select>
					</div>
				</div>
			{/if}
			{#if type == 'absolute'}
				<div class="flex gap-2">
					<div class="grow">
						From: <input
							type="datetime-local"
							placeholder="Start date"
							class="input input-sm input-bordered w-full max-w-xs"
							on:change={(event) => {
								from = new Date(event.currentTarget.value);
							}}
							value={toLocalDateStr(from)}
						/>
					</div>
					<div class="grow">
						To: <input
							type="datetime-local"
							placeholder="End date"
							class="input input-sm input-bordered w-full max-w-xs"
							on:change={(event) => {
								to = new Date(event.currentTarget.value);
							}}
							value={toLocalDateStr(to)}
						/>
					</div>
				</div>
			{/if}
			<div class="flex justify-end gap-2">
				<button
					class="btn btn-sm btn-ghost"
					on:click={() => {
						reset();
						details.removeAttribute('open');
						open = false;
					}}>Cancel</button
				>
				<button
					class="btn btn-sm btn-success"
					on:click={() => {
						switch (type) {
							case 'absolute':
								range.set({
									type,
									from,
									to
								});
								break;
							case 'relative':
								range.set({
									type,
									amount,
									unit
								});
						}
						details.removeAttribute('open');
						open = false;
					}}>Select</button
				>
			</div>
		</div>
	</div>
</details>
{#if open}
	<!-- svelte-ignore a11y-no-static-element-interactions -->
	<!-- svelte-ignore a11y-click-events-have-key-events -->
	<div
		class="w-screen h-screen bottom-0 left-0 fixed bg-salte"
		on:click={() => {
			details.removeAttribute('open');
			open = false;
		}}
	></div>
{/if}
