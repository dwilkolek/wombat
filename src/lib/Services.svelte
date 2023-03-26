<script lang="ts">
  import { state } from "./store";
  let services = state.services;
</script>

<table class="table w-full">
  <thead>
    <tr>
      <th />
      <th>Name</th>
    </tr>
  </thead>
  <tbody>
    {#each $services as service, i}
      <tr>
        <th>{i}</th>
        <td>{service.split("/").at(-1)}</td>
      </tr>
    {/each}
  </tbody>
</table>
