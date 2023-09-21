<script lang="ts">
	import { userStore } from '$lib/stores/user-store';

	import { clusterStore } from '$lib/stores/cluster-store';
	import { serviceStore } from '$lib/stores/service-store';
	import { invoke } from '@tauri-apps/api';
	import { writable } from 'svelte/store';
	import { listen } from '@tauri-apps/api/event';

	$: activeCluser = clusterStore.activeCluser;

	$: services = serviceStore.getServices($activeCluser);

	$: selectedService = serviceStore.selectedService;

	$: clusters = clusterStore.clusters;
	const toLocalDateStr = (date: Date) => {
		return `${date.getUTCFullYear()}-${withZeros(date.getMonth() + 1)}-${withZeros(
			date.getDate()
		)}T${withZeros(date.getHours())}:${withZeros(date.getMinutes())}:${withZeros(
			date.getSeconds()
		)}`;
	};
	const withZeros = (v: number) => {
		return v < 10 ? `0${v}` : v;
	};
	const startDate = writable<Date>(new Date(new Date().getTime() - 24 * 60 * 60 * 1000));
	const endDate = writable<Date>(new Date());
	const filterString = writable<String>('{$.level = "ERROR"}');

	const logs = writable<{ message: string }[]>([]);

	listen<string>('new-log-found', (newLog) => {
		let messageObj = JSON.parse(newLog.payload);
		console.log(messageObj);
		if (messageObj === newLog.payload) {
			logs.update((n) => [
				...n,
				{
					message: newLog.payload
				}
			]);
		} else {
			logs.update((n) => [
				...n,
				{
					message: messageObj.message
				}
			]);
		}
	});
</script>

<svelte:head>
	<title>Logs</title>
	<meta name="description" content="Wombat" />
</svelte:head>
<div class="flex gap-2 content-end">
	<div class="flex flex-col gap-2">
		<div class="flex gap-2">
			<div>
				<select class="select select-bordered" bind:value={$activeCluser}>
					{#each $clusters as cluster}
						<option value={cluster}>{cluster.name}</option>
					{/each}
				</select>
			</div>

			<div>
				<select class="select select-bordered" bind:value={$selectedService}>
					{#await services then services}
						{#each services as service}
							<option value={service}>{service.name}</option>
						{/each}
					{/await}
				</select>
			</div>

			<div>
				<input
					type="datetime-local"
					placeholder="Start date"
					class="input input-bordered w-full max-w-xs"
					on:change={(event) => {
						startDate.set(new Date(event.currentTarget.value))
					}}
					value={toLocalDateStr($startDate)}
				/>
			</div>
			<div>
				<input
					type="datetime-local"
					placeholder="End date"
					class="input input-bordered w-full max-w-xs"
					on:change={(event) => {
						endDate.set(new Date(event.currentTarget.value))
					}}
					value={toLocalDateStr($endDate)}
				/>
			</div>
		</div>
		<div class="w-full">
			<input
				type="text"
				placeholder="Filter"
				class="input input-bordered w-full"
				bind:value={$filterString}
			/>
		</div>
	</div>
	<div class="flex items-center h-full self-end">
		<button
			class="btn btn-active btn-primary"
			disabled={!$selectedService}
			on:click={() => {
				logs.set([]);
				invoke('find_logs', {
					app: $selectedService?.name,
					env: $selectedService?.env,
					start: $startDate.getTime(),
					end: $endDate.getTime(),
					filter: $filterString
				});
			}}>Search!</button
		>
	</div>
	
</div>
<div class="flex flex-col w-full">
	{#each $logs as log}
		<div class="grow">{JSON.stringify(log)}</div>
	{/each}
</div>
