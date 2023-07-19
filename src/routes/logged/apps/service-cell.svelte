<script lang="ts">
	import ServiceProxyStartBtn from '$lib/service-proxy-start-btn.svelte';
	import ServiceProxyStopBtn from '$lib/service-proxy-stop-btn.svelte';

	import { taskStore } from '$lib/task-store';
	import type { ServiceDetails } from '$lib/types';

	export let service: ServiceDetails;
	$: port = $taskStore?.find((t) => t.arn == service?.arn)?.port;
</script>

<div class="flex flex-row gap-2 items-start pr-4">
	{#if port}
		<ServiceProxyStopBtn {service} {port} />
	{/if}
	{#if !port}
		<ServiceProxyStartBtn {service} />
	{/if}
</div>
