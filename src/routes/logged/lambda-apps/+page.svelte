<script lang="ts">
	import LambdaAppCard from '$lib/components/lambda-app-card.svelte';
	const lambdaServices = ['filevault', 'commenting-service', 'feedback-form', 'ad-user-meta'];
</script>

<svelte:head>
	<title>Lambda Apps</title>
	<meta name="description" content="Wombat" />
</svelte:head>
<div class="m-2 flex items-center">
	<div role="alert" class="alert shadow-lg p-2">
		<svg
			xmlns="http://www.w3.org/2000/svg"
			class="stroke-warning shrink-0 h-6 w-6"
			fill="none"
			viewBox="0 0 24 24"
			><path
				stroke-linecap="round"
				stroke-linejoin="round"
				stroke-width="2"
				d="M12 9v2m0 4h.01m-6.938 4h13.856c1.54 0 2.502-1.667 1.732-3L13.732 4c-.77-1.333-2.694-1.333-3.464 0L3.34 16c-.77 1.333.192 3 1.732 3z"
			/></svg
		>
		<div>
			<h3 class="font-bold">This is BETA feature and requires installed chrome extension.</h3>
			<div class="text-xs">
				To have always valid session for specific environment you have to have open any app that do
				some polling eg. for comments.
			</div>
		</div>
	</div>
</div>
<div class="bg-base-100 flex flex-row justify-between px-2 sticky top-[68px] z-40">
	<div class="flex flex-wrap gap-2">
		{#each lambdaServices as app (app)}
			<LambdaAppCard {app} />
		{/each}
	</div>
</div>
