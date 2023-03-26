<script lang="ts">
  import Menu from "./lib/Menu.svelte";
  import Services from "./lib/Services.svelte";
</script>

<main class="min-h-screen">
  <Menu />

  <div class="mx-auto">
    <div class="flex gap-2">
      <div class="card w-96 bg-neutral text-neutral-content">
        <div class="card-body items-center text-center">
          <h2 class="card-title">Services</h2>
          <Services />
        </div>
      </div>
      <div class="card w-96 bg-neutral text-neutral-content">
        <div class="card-body items-center text-center">
          <h2 class="card-title">Databases</h2>
          <Services />
        </div>
      </div>
    </div>
  </div>
</main>

<style>
</style>
