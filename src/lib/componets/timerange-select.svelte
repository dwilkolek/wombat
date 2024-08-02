<script lang="ts">
	import type { Timerange } from '$lib/types';
	import { endOfDay, format, startOfDay } from 'date-fns';

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

	export let range: Timerange = {
		type: 'relative',
		amount: 30,
		unit: 'minutes'
	};
	export let onSelect: (range: Timerange) => void = () => {};
	let tempRange: Timerange = { ...range };
	let details: HTMLDetailsElement;
</script>

<details class="dropdown grow" bind:this={details}>
	<summary class="btn btn-sm w-[450px]">
		{formatTimerange(range)}
	</summary>
	<div class="menu dropdown-content bg-base-300 rounded-box z-[1] p-2 shadow w-[450px] -my-8">
		<!-- <div role="tablist" class="tabs tabs-boxed">
			<a role="tab" class={`tab ${range.type == 'relative' ? 'tab-active' : ''}`}>Relative</a>
			<a role="tab" class={`tab ${range.type == 'absolute' ? 'tab-active' : ''}`}>Absolute</a>
		</div> -->
		<div class="flex flex-col gap-2">
			<div class="flex gap-2">
				<button
					class={`btn btn-sm ${tempRange.type == 'relative' ? 'btn-primary' : 'btn-ghost'}`}
					on:click={() => {
						tempRange = {
							type: 'relative',
							amount: 30,
							unit: 'minutes'
						};
					}}>Relative</button
				>
				<button
					class={`btn btn-sm ${tempRange.type == 'absolute' ? 'btn-primary' : 'btn-ghost'}`}
					on:click={() => {
						tempRange = {
							type: 'absolute',
							from: startOfDay(new Date()),
							to: endOfDay(new Date())
						};
					}}>Absolute</button
				>
			</div>
			{#if tempRange.type == 'relative'}
				<div class="flex gap-2">
					<div class="grow">
						Amount: <input
							type="number"
							class="input input-sm input-bordered w-full"
							on:change={(event) => {
								if (tempRange.type == 'relative') {
									tempRange = { ...tempRange, amount: parseInt(event.currentTarget.value) };
								}
							}}
							value={tempRange.amount}
						/>
					</div>
					<div class="grow">
						Unit: <select
							class="input input-sm input-bordered w-full"
							on:change={(event) => {
								if (tempRange.type == 'relative') {
									tempRange = { ...tempRange, unit: event.currentTarget.value };
								}
							}}
						>
							<option value="minutes" selected={tempRange.unit == 'minutes'}>Minute</option>
							<option value="hours" selected={tempRange.unit == 'hours'}>Hour</option>
						</select>
					</div>
				</div>
			{/if}
			{#if tempRange.type == 'absolute'}
				<div class="flex gap-2">
					<div class="grow">
						From: <input
							type="datetime-local"
							placeholder="Start date"
							class="input input-sm input-bordered w-full max-w-xs"
							on:change={(event) => {
								if (tempRange.type == 'absolute') {
									tempRange = { ...tempRange, from: new Date(event.currentTarget.value) };
								}
							}}
							value={toLocalDateStr(tempRange.from)}
						/>
					</div>
					<div class="grow">
						To: <input
							type="datetime-local"
							placeholder="End date"
							class="input input-sm input-bordered w-full max-w-xs"
							on:change={(event) => {
								if (tempRange.type == 'absolute') {
									tempRange = { ...tempRange, to: new Date(event.currentTarget.value) };
								}
							}}
							value={toLocalDateStr(tempRange.to)}
						/>
					</div>
				</div>
			{/if}
			<div class="flex justify-end gap-2">
				<button
					class="btn btn-sm btn-ghost"
					on:click={() => {
						details.removeAttribute('open');
					}}>Cancel</button
				>
				<button
					class="btn btn-sm btn-success"
					on:click={() => {
						details.removeAttribute('open');
						onSelect(tempRange);
					}}>Select</button
				>
			</div>
		</div>
	</div>
</details>
