<script lang="ts">
	import ServiceProxyStartBtn from './service-proxy-start-btn.svelte';
	import ServiceProxyStopBtn from './service-proxy-stop-btn.svelte';

	import { taskStore } from '$lib/stores/task-store';
	import type { EcsService } from '$lib/types';

	interface Props {
		service: EcsService;
	}

	let { service }: Props = $props();
	let port = $derived($taskStore?.find((t) => t.arn == service?.arn)?.port);
</script>

{#if port}
	<ServiceProxyStopBtn service_arn={service.arn} />
{/if}
{#if !port}
	<ServiceProxyStartBtn {service} />
{/if}
