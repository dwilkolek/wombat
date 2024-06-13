<script lang="ts">
	import LambdaAppCard from '$lib/componets/lambda-app-card.svelte';
	const lambdaServices = ['filevault', 'commenting-service', 'feedback-form'];
</script>

<svelte:head>
	<title>Lambda Apps</title>
	<meta name="description" content="Wombat" />
</svelte:head>

<div class="bg-base-100 flex flex-row justify-between px-2 sticky top-[68px] z-50">
	<div class="flex flex-wrap gap-2">
		{#each lambdaServices as app}
			<LambdaAppCard {app} />
		{/each}
	</div>
</div>
