<script lang="ts">
	import ServiceProxyStartBtn from './service-proxy-start-btn.svelte';
	import ServiceProxyStopBtn from './service-proxy-stop-btn.svelte';

	import { taskStore } from '$lib/stores/task-store';
	import type { ServiceDetails } from '$lib/types';

	export let service: ServiceDetails;
	$: port = $taskStore?.find((t) => t.arn == service?.arn)?.port;
</script>

{#if port}
	<ServiceProxyStopBtn {service} {port} />
{/if}
{#if !port}
	<ServiceProxyStartBtn {service} />
{/if}
