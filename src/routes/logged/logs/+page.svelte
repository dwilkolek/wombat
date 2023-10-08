<script lang="ts">
	import { userStore } from '$lib/stores/user-store';
	import { JsonView } from '@zerodevx/svelte-json-view';
	import { endOfDay, format, startOfDay, sub } from 'date-fns';
	import { clusterStore } from '$lib/stores/cluster-store';
	import { serviceStore } from '$lib/stores/service-store';
	import { invoke } from '@tauri-apps/api';
	import { writable } from 'svelte/store';
	import { listen } from '@tauri-apps/api/event';
	import { writeText } from '@tauri-apps/api/clipboard';
	import { beforeNavigate } from '$app/navigation';

	$: activeCluser = clusterStore.activeCluser;
	$: tracked_names = $userStore.tracked_names;
	$: services = serviceStore.getServices($activeCluser).then((services) => [
		services.filter((a) => {
			return tracked_names.includes(a.name);
		}),
		services.filter((a) => {
			return !tracked_names.includes(a.name);
		})
	]);

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
	const selectedLog = writable<any>(undefined);
	const startDate = writable<Date>(new Date(new Date().getTime() - 24 * 60 * 60 * 1000));
	const endDate = writable<Date>(new Date());
	const filterString = writable<String>('');
	let showLogDetails = false;
	type LogEntry = {
		log_stream_name: string;
		timestamp: number;
		ingestion_time: number;
		message: string;
	};
	type LogStyle = {
		bg: string;
		active: string;
		hover: string;
	};
	type LogLevel = 'INFO' | 'WARN' | 'ERROR' | 'TRACE' | 'DEBUG' | 'UNKNOWN';
	type UiLogEntry = {
		id: number;
		message: string;
		level: LogLevel;
		timestamp: number;
		data: any;
		style: LogStyle;
	};
	let isLookingForLogs = false;
	let searchError: string | undefined = undefined;

	let searchStatus: 'success' | 'error' | 'aborted' | undefined = undefined;

	const logs = writable<UiLogEntry[]>([]);
	function logStyle(level: LogLevel): LogStyle {
		switch (level) {
			case 'INFO':
				return {
					bg: 'bg-cyan-900',
					hover: 'hover:bg-cyan-800',
					active: '!bg-cyan-700'
				};
			case 'WARN':
				return {
					bg: 'bg-amber-900',
					hover: 'hover:bg-amber-800',
					active: '!bg-amber-700'
				};
			case 'ERROR':
				return {
					bg: 'bg-red-900',
					hover: 'hover:bg-red-800',
					active: '!bg-red-700'
				};
			case 'TRACE':
				return {
					bg: 'bg-fuchsia-900',
					hover: 'hover:bg-fuchsia-800',
					active: '!bg-fuchsia-700'
				};
			case 'DEBUG':
				return {
					bg: 'bg-emerald-900',
					hover: 'hover:bg-emerald-800',
					active: '!bg-emerald-700'
				};
			case 'UNKNOWN':
				return {
					bg: 'bg-gray-900',
					hover: 'hover:bg-gray-800',
					active: '!bg-gray-700'
				};
		}
	}
	function processLogs(newLogs: LogEntry[]) {
		logs.update((n) => {
			let startIndex = n.length;
			let newLogArr = newLogs.map((log) => ({ id: startIndex++, ...transformLog(log) }));
			return [...n, ...newLogArr];
		});
	}
	function transformLog(newLog: LogEntry) {
		let isString;
		try {
			if (typeof newLog.message == 'object') {
				isString = false;
			} else {
				isString = true;
				if (typeof JSON.parse(newLog.message) === 'object') {
					isString = false;
				} else {
					isString = true;
				}
			}
		} catch (e) {
			isString = true;
		}
		if (isString) {
			const level = (newLog.message.match(/(INFO|WARN|ERROR|DEBUG|TRACE)/)?.[0] ??
				'UNKNOWN') as LogLevel;
			return {
				timestamp: newLog.timestamp,
				level,
				message: newLog.message,
				data: { message: newLog.message },
				style: logStyle(level)
			};
		} else {
			const logData = JSON.parse(newLog.message);
			const level = logData?.level?.match(/(INFO|WARN|ERROR|DEBUG|TRACE)/)?.[0] ?? 'UNKNOWN';
			return {
				timestamp: newLog.timestamp,
				level,
				message: logData.message,
				data: logData,
				style: logStyle(level)
			};
		}
	}

	listen<LogEntry[]>('new-log-found', (event) => processLogs(event.payload));

	listen('find-logs-success', () => {
		isLookingForLogs = false;
		searchStatus = 'success';
	});
	listen<string>('find-logs-error', (event) => {
		isLookingForLogs = false;
		searchStatus = 'error';
		searchError = event.payload;
	});
	beforeNavigate(async () => {
		invoke('abort_find_logs', { reason: 'navigation' });
	});
</script>

<svelte:head>
	<title>Logs</title>
	<meta name="description" content="Wombat" />
</svelte:head>

<div class="flex gap-8 content-end flex-col-reverse lg:flex-row px-2">
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
						<option value={undefined}> -- favorite -- </option>
						{#each services[0] as service}
							<option value={service} selected={$selectedService?.arn === service.arn}
								>{service.name}</option
							>
						{/each}
						<option value={undefined}> -- rest -- </option>
						{#each services[1] as service}
							<option value={service} selected={$selectedService?.arn === service.arn}
								>{service.name}</option
							>
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
						startDate.set(new Date(event.currentTarget.value));
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
						endDate.set(new Date(event.currentTarget.value));
					}}
					value={toLocalDateStr($endDate)}
				/>
			</div>
		</div>
		<div class="w-full flex gap-2">
			<input
				type="text"
				placeholder="Filter"
				autocomplete="off"
				autocorrect="off"
				autocapitalize="off"
				spellcheck="false"
				class="input input-bordered grow"
				bind:value={$filterString}
			/>
			{#if !isLookingForLogs}
				<button
					class="btn btn-active btn-primary"
					disabled={!$selectedService || $selectedService?.env !== $activeCluser.env}
					on:click={() => {
						isLookingForLogs = true;
						searchError = undefined;
						logs.set([]);
						selectedLog.set(undefined);
						showLogDetails = false;
						searchStatus = undefined;
						invoke('find_logs', {
							app: $selectedService?.name,
							env: $activeCluser?.env,
							start: $startDate.getTime(),
							end: $endDate.getTime(),
							filter: $filterString
						});
					}}
				>
					<svg
						xmlns="http://www.w3.org/2000/svg"
						fill="none"
						viewBox="0 0 24 24"
						stroke-width="1.5"
						stroke="currentColor"
						class="w-6 h-6"
					>
						<path
							stroke-linecap="round"
							stroke-linejoin="round"
							d="M21 21l-5.197-5.197m0 0A7.5 7.5 0 105.196 5.196a7.5 7.5 0 0010.607 10.607z"
						/>
					</svg>

					Search!</button
				>
			{/if}
			{#if isLookingForLogs}
				<button
					class="btn btn-active btn-warning"
					on:click={() => {
						invoke('abort_find_logs', { reason: 'user-request' });
						isLookingForLogs = false;
						searchStatus = 'aborted';
					}}
				>
					<svg
						xmlns="http://www.w3.org/2000/svg"
						fill="none"
						viewBox="0 0 24 24"
						stroke-width="1.5"
						stroke="currentColor"
						class="w-6 h-6"
					>
						<path
							stroke-linecap="round"
							stroke-linejoin="round"
							d="M15.75 5.25v13.5m-7.5-13.5v13.5"
						/>
					</svg>

					Abort</button
				>
			{/if}
		</div>
	</div>
	<div class="flex h-full flex-col gap-2">
		Ping me for custom search filter templates.
		<div class="flex flex-wrap gap-2">
			<button
				class="btn btn-accent btn-xs"
				on:click={() => {
					startDate.set(sub(new Date(), { minutes: 5 }));
					endDate.set(new Date());
				}}>Last 5m</button
			>
			<button
				class="btn btn-accent btn-xs"
				on:click={() => {
					startDate.set(sub(new Date(), { minutes: 15 }));
					endDate.set(new Date());
				}}>Last 15m</button
			>
			<button
				class="btn btn-accent btn-xs"
				on:click={() => {
					startDate.set(sub(new Date(), { minutes: 30 }));
					endDate.set(new Date());
				}}>Last 30m</button
			>
			<button
				class="btn btn-accent btn-xs"
				on:click={() => {
					startDate.set(sub(new Date(), { hours: 1 }));
					endDate.set(new Date());
				}}>Last 1h</button
			>
			<button
				class="btn btn-accent btn-xs"
				on:click={() => {
					startDate.set(sub(new Date(), { hours: 4 }));
					endDate.set(new Date());
				}}>Last 4h</button
			>
			<button
				class="btn btn-accent btn-xs"
				on:click={() => {
					startDate.set(sub(new Date(), { hours: 24 }));
					endDate.set(new Date());
				}}>Last 24h</button
			>
			<button
				class="btn btn-accent btn-xs"
				on:click={() => {
					startDate.set(startOfDay(new Date()));
					endDate.set(endOfDay(new Date()));
				}}>Today</button
			>
		</div>
		<div class="flex flex-wrap gap-2">
			{#if $selectedService?.name == 'rome'}
				<button
					class="btn btn-active btn-secondary btn-xs"
					on:click={() => {
						filterString.set(`{ $.level = "ERROR" }`);
					}}>Only Errors</button
				>
				<button
					class="btn btn-active btn-secondary btn-xs"
					on:click={() => {
						filterString.set(`{ $.mdc.traceId = "TRACE_ID_UUID" }`);
					}}>By Trace</button
				>
			{/if}
		</div>
	</div>
</div>
<div class="p-1 min-h-6">
	{#if !!searchError}{searchError}{/if}
	{#if isLookingForLogs}<progress class={`progress w-full ${searchError ? 'progress-error' : ''}`}
		></progress>{/if}
	{#if !isLookingForLogs}<progress
			class={`progress w-full 
				${searchStatus == 'aborted' ? 'progress-warning' : ''} 
				${searchStatus == 'success' ? 'progress-success' : ''} 
				${searchStatus == 'error' ? 'progress-error' : ''}`}
			value="100"
			max="100"
		></progress>{/if}
</div>
<div class="flex flex-col w-full gap-2">
	<div
		class={`overflow-auto ${
			showLogDetails ? 'h-[calc(60vh-260px)]' : 'h-[calc(100vh-260px)]'
		} w-full`}
	>
		<table class="table table-xs w-full">
			<tbody>
				{#each $logs as log}
					<tr
						on:click={() => {
							selectedLog.update((c) => (c === log.data ? undefined : log.data));
							if ($selectedLog) {
								showLogDetails = true;
							} else {
								showLogDetails = false;
							}
						}}
						class={`cursor-pointer text-white ${log.style.bg} ${log.style.hover} ${
							$selectedLog === log.data ? log.style.active : ''
						}`}
					>
						<td>{log.level}</td>
						<td class="min-w-[200px] max-w-[200px]"
							>{format(new Date(log.timestamp), 'yyyy-MM-dd HH:mm:ss.SSS')}</td
						>
						<td class="w-full">{log.message}</td>
					</tr>
				{/each}
			</tbody>
		</table>
	</div>
</div>
<div class="fixed w-full bottom-0">
	<div class="w-full flex-col bg-base-300 rounded-t-lg">
		<div class="w-full flex justify-center">
			{#if !showLogDetails}
				<button on:click={() => (showLogDetails = true)}>
					<svg
						xmlns="http://www.w3.org/2000/svg"
						fill="none"
						viewBox="0 0 24 24"
						stroke-width="1.5"
						stroke="currentColor"
						class="w-6 h-6"
					>
						<path stroke-linecap="round" stroke-linejoin="round" d="M4.5 15.75l7.5-7.5 7.5 7.5" />
					</svg>
				</button>
			{/if}
			{#if showLogDetails}
				<button on:click={() => (showLogDetails = false)}>
					<svg
						xmlns="http://www.w3.org/2000/svg"
						fill="none"
						viewBox="0 0 24 24"
						stroke-width="1.5"
						stroke="currentColor"
						class="w-6 h-6"
					>
						<path stroke-linecap="round" stroke-linejoin="round" d="M19.5 8.25l-7.5 7.5-7.5-7.5" />
					</svg>
				</button>
			{/if}
		</div>

		{#if $selectedLog && showLogDetails}
			<div class="h-[40vh] flex flex-col gap-2">
				<button
					class="m-2 btn btn-active btn-primary btn-sm"
					on:click={async () => {
						await writeText(JSON.stringify($selectedLog, null, 2));
					}}>Copy raw json</button
				>
				<div class="text-sm overflow-auto h-[calc(40vh-80px)]">
					<JsonView json={$selectedLog} />
				</div>
			</div>
		{/if}
		{#if !$selectedLog && showLogDetails}
			<div class="overflow-auto h-[40vh] justify-evenly text-center flex flex-col gap-2">
				Select log to see details
			</div>
		{/if}
	</div>
</div>
