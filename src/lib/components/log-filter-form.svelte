<script lang="ts">
	import { logFiltersStore } from '$lib/stores/log-filters-store';
	import type { LogFilter } from '$lib/types';
	import { get } from 'svelte/store';
	import MultiSelect from './multi-select.svelte';
	import { servicedForActiveCluster } from '$lib/stores/service-store';

	let filters = $state<LogFilter[]>([]);
	let initialized = false;

	logFiltersStore.subscribe((store) => {
		if (store.filters && !store.isLoading && !initialized) {
			filters = JSON.parse(JSON.stringify(store.filters)); // Deep copy
			initialized = true;
		}
	});

	let error = $state('');

	function save() {
		try {
			get(logFiltersStore).save(filters);
			error = '';
		} catch (e: any) {
			error = e.message;
		}
	}

	function addFilter() {
		const maxId = filters.reduce((max, f) => Math.max(max, f.id), 0);
		filters.push({
			id: maxId + 1,
			label: 'New Filter',
			filter: '',
			services: []
		});
	}

	function removeFilter(index: number) {
		filters.splice(index, 1);
	}
</script>

<div class="flex flex-col gap-4">
	{#each filters as filter, i}
		<div class="card bg-base-100 shadow-sm border border-base-300">
			<div class="card-body p-4 gap-2">
				<div class="flex justify-between items-center">
					<h3 class="font-bold">Filter #{filter.id}</h3>
					<button
						class="btn btn-ghost btn-xs text-error"
						onclick={() => removeFilter(i)}
						aria-label="Remove filter"
					>
						<svg
							xmlns="http://www.w3.org/2000/svg"
							fill="none"
							viewBox="0 0 24 24"
							stroke-width="1.5"
							stroke="currentColor"
							class="size-5"
						>
							<path
								stroke-linecap="round"
								stroke-linejoin="round"
								d="m14.74 9-.346 9m-4.788 0L9.26 9m9.968-3.21c.342.052.682.107 1.022.166m-1.022-.165L18.16 19.673a2.25 2.25 0 0 1-2.244 2.077H8.084a2.25 2.25 0 0 1-2.244-2.077L4.772 5.79m14.456 0a48.108 48.108 0 0 0-3.478-.397m-12 .562c.34-.059.68-.114 1.022-.165m0 0a48.11 48.11 0 0 1 3.478-.397m7.5 0v-.916c0-1.18-.91-2.164-2.09-2.201a51.964 51.964 0 0 0-3.32 0c-1.18.037-2.09 1.022-2.09 2.201v.916m7.5 0a48.667 48.667 0 0 0-7.5 0"
							/>
						</svg>
					</button>
				</div>

				<div class="form-control">
					<label class="label hidden" for={`label-${i}`}>
						<span class="label-text">Label</span>
					</label>
					<input
						id={`label-${i}`}
						type="text"
						placeholder="Label (e.g. By Trace)"
						class="input input-sm input-bordered w-full"
						bind:value={filter.label}
					/>
				</div>

				<div class="form-control">
					<label class="label hidden" for={`filter-${i}`}>
						<span class="label-text">Filter Expression</span>
					</label>
					<input
						id={`filter-${i}`}
						type="text"
						placeholder={'Filter Expression (e.g. { $.level = "ERROR" })'}
						class="input input-sm input-bordered w-full font-mono"
						bind:value={filter.filter}
					/>
				</div>

				<div class="form-control">
					<label class="label hidden" for={`services-${i}`}>
						<span class="label-text">Services (comma separated, empty for all)</span>
					</label>
					<MultiSelect
						items={$servicedForActiveCluster.map((s) => s.name).sort()}
						bind:selected={filter.services}
						placeholder="Select services..."
					/>
				</div>
			</div>
		</div>
	{/each}

	<button class="btn btn-outline btn-sm btn-block" onclick={addFilter}> + Add Filter </button>

	{#if error}
		<div class="text-error mt-2">{error}</div>
	{/if}

	<button class="btn btn-primary mt-4" onclick={save} disabled={$logFiltersStore.isLoading}>
		{#if $logFiltersStore.isLoading}
			Saving...
		{:else}
			Save Log Filters
		{/if}
	</button>
</div>
