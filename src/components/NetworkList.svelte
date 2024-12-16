<script lang="ts">
    import { createEventDispatcher } from "svelte";
    import type { WiFiNetwork } from "../utils/api";

    export let networks: WiFiNetwork[] = [];

    const dispatch = createEventDispatcher<{
        selectNetwork: WiFiNetwork;
    }>();

    function selectNetwork(network: WiFiNetwork): void {
        dispatch("selectNetwork", network);
    }

    function getSignalLevel(signalQuality: number): number {
        if (signalQuality >= 80) return 4;
        if (signalQuality >= 60) return 3;
        if (signalQuality >= 40) return 2;
        return 1;
    }

    function getSignalColor(signalQuality: number): string {
        if (signalQuality >= 80) return "#4caf50"; // excellent
        if (signalQuality >= 60) return "#8bc34a"; // good
        if (signalQuality >= 40) return "#ffc107"; // fair
        return "#ff5722"; // poor
    }

    function formatFrequency(frequency: number | undefined): string {
        return frequency ? `${frequency.toFixed(3)} GHz` : "N/A";
    }

    function formatResponseTime(time: number | undefined): string {
        return time ? `${time.toFixed(2)} ms` : "N/A";
    }
</script>

<div class="network-list">
    <h2 class="text-xl font-semibold mb-4">Detected Networks</h2>
    {#each networks as network}
        <div class="network-item" role="button" tabindex="0" on:click={() => selectNetwork(network)} on:keydown={(e) => e.key === 'Enter' && selectNetwork(network)}>
            <div class="network-main">
                <svg
                    class="wifi-icon"
                    viewBox="0 0 24 24"
                    width="24"
                    height="24"
                >
                    <path
                        d="M1 9l2 2c4.97-4.97 13.03-4.97 18 0l2-2C16.93 2.93 7.08 2.93 1 9zm8 8l3 3 3-3c-1.65-1.66-4.34-1.66-6 0zm-4-4l2 2c2.76-2.76 7.24-2.76 10 0l2-2C15.14 9.14 8.87 9.14 5 13z"
                        fill="#e0e0e0"
                    />
                    <path
                        d="M1 9l2 2c4.97-4.97 13.03-4.97 18 0l2-2C16.93 2.93 7.08 2.93 1 9zm8 8l3 3 3-3c-1.65-1.66-4.34-1.66-6 0zm-4-4l2 2c2.76-2.76 7.24-2.76 10 0l2-2C15.14 9.14 8.87 9.14 5 13z"
                        fill={getSignalColor(network.signal_quality)}
                        clip-path={`polygon(0 ${100 - getSignalLevel(network.signal_quality) * 25}%, 100% ${100 - getSignalLevel(network.signal_quality) * 25}%, 100% 100%, 0 100%)`}
                    />
                </svg>
                <span class="ssid">{network.ssid}</span>
                <span class="channel">Ch: {network.channel ?? "N/A"}</span>
            </div>
            <div class="network-details">
                <span class="mac-address">{network.bssid}</span>
                <span class="frequency-response">
                    {formatFrequency(network.frequency)} / {formatResponseTime(
                        network.avg_response_time
                    )}
                </span>
                <svg
                    class="lock-icon"
                    viewBox="0 0 24 24"
                    width="16"
                    height="16"
                >
                    {#if network.security !== "Open"}
                        <path
                            d="M18 8h-1V6c0-2.76-2.24-5-5-5S7 3.24 7 6v2H6c-1.1 0-2 .9-2 2v10c0 1.1.9 2 2 2h12c1.1 0 2-.9 2-2V10c0-1.1-.9-2-2-2zm-6 9c-1.1 0-2-.9-2-2s.9-2 2-2 2 .9 2 2-.9 2-2 2zm3.1-9H8.9V6c0-1.71 1.39-3.1 3.1-3.1 1.71 0 3.1 1.39 3.1 3.1v2z"
                            fill="currentColor"
                        />
                    {:else}
                        <path
                            d="M12 17c1.1 0 2-.9 2-2s-.9-2-2-2-2 .9-2 2 .9 2 2 2zm6-9h-1V6c0-2.76-2.24-5-5-5S7 3.24 7 6h2c0-1.66 1.34-3 3-3s3 1.34 3 3v2H6c-1.1 0-2 .9-2 2v10c0 1.1.9 2 2 2h12c1.1 0 2-.9 2-2V10c0-1.1-.9-2-2-2zm0 12H6V10h12v10z"
                            fill="currentColor"
                        />
                    {/if}
                </svg>
            </div>
        </div>
    {/each}
</div>

<style>
    .network-list {
        @apply w-96 bg-gray-700 p-4 rounded-lg;
    }

    .network-item {
        @apply flex flex-col p-2 cursor-pointer hover:bg-gray-600 rounded transition-colors duration-200 mb-2;
    }

    .network-main {
        @apply flex items-center w-full;
    }

    .network-details {
        @apply flex items-center justify-between mt-1 text-xs text-gray-300;
    }

    .wifi-icon {
        @apply w-6 h-6 mr-2 flex-shrink-0;
    }

    .ssid {
        @apply flex-grow text-white truncate;
    }

    .channel {
        @apply text-gray-300 text-sm ml-2;
    }

    .mac-address {
        @apply font-mono;
    }

    .frequency-response {
        @apply mx-2;
    }

    .lock-icon {
        @apply w-4 h-4 flex-shrink-0;
    }
</style>
