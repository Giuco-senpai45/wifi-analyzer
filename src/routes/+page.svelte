<script lang="ts">
    import { onMount } from "svelte";
    import NetworkList from "../components/NetworkList.svelte";
    import ChannelRating from "../components/ChannelRating.svelte";
    import ChannelGraph from "../components/ChannelGraph.svelte";
    import { scanWifi, getChannelData } from "../utils/api";
    import type { WiFiNetwork, ChannelData } from "../utils/api";
    import NetworkDetails from "../components/NetworkDetails.svelte";
    import PacketSniffer from "../components/PacketSniffer.svelte";
    import NetworkScanProgress from "../components/NetworkScanProgress.svelte";
    import { listen } from "@tauri-apps/api/event";

    let networks: WiFiNetwork[] = [];
    let selectedNetwork: WiFiNetwork | null = null;
    let channelData: ChannelData[] = [];
    let isNetworkDetailsVisible = false;
    let isScanning = false;

    // Keep track of unique networks by BSSID
    const uniqueNetworks = new Map<string, WiFiNetwork>();

    async function performScan(): Promise<void> {
        if (isScanning) return;
        isScanning = true;
        networks = [];
        uniqueNetworks.clear();

        try {
            // Set up event listener for scan progress
            const unlisten = await listen<WiFiNetwork[]>(
                "wifi_scan_progress",
                (event) => {
                    console.log(
                        "Scan progress update received:",
                        event.payload,
                    );
                    // Update the unique networks map
                    event.payload.forEach((network) => {
                        uniqueNetworks.set(network.bssid, network);
                    });
                    // Convert map to array and update the UI
                    networks = Array.from(uniqueNetworks.values());
                    // Trigger Svelte reactivity
                    networks = networks;
                },
            );

            // Start the scan
            const finalNetworks = await scanWifi();
            console.log("Final scan results:", finalNetworks);

            // Update with final results if they exist
            if (finalNetworks && finalNetworks.length > 0) {
                finalNetworks.forEach((network) => {
                    uniqueNetworks.set(network.bssid, network);
                });
                networks = Array.from(uniqueNetworks.values());
            }

            // Get channel data
            if (networks.length > 0) {
                channelData = await getChannelData(networks);
            }

            unlisten();
        } catch (error) {
            console.error("Failed to update data:", error);
        } finally {
            isScanning = false;
        }
    }

    onMount(() => {
        performScan();
    });

    function selectNetwork(network: WiFiNetwork): void {
        console.log("Selected network:", network);
        selectedNetwork = network;
        isNetworkDetailsVisible = true;
        updateChannelData();
    }

    async function updateChannelData(): Promise<void> {
        if (selectedNetwork) {
            channelData = await getChannelData([selectedNetwork]);
        }
    }

    function toggleNetworkDetails(): void {
        isNetworkDetailsVisible = false;
        selectedNetwork = null;
    }

    $: {
        getChannelData(networks).then((data) => {
            channelData = data;
        });
    }
</script>

<main class="min-h-screen bg-gray-800 text-white p-6">
    <h1 class="text-3xl font-bold mb-6">WiFi Analyzer</h1>
    <div class="analyzer">
        {#if isNetworkDetailsVisible}
            <div class="network-details-container">
                <button class="back-button" on:click={toggleNetworkDetails}
                    >Back</button
                >
                <ChannelRating {channelData} />
                <ChannelGraph {channelData} />
                <NetworkDetails {channelData} />
            </div>
        {:else}
            <div class="grid grid-cols-2">
                <button
                    class="col-span-2 p-2 bg-green-800 hover:bg-green-600 text-white rounded mb-4 disabled:opacity-50 disabled:cursor-not-allowed"
                    on:click={performScan}
                    disabled={isScanning}
                >
                    {isScanning ? "Scanning..." : "Re-scan networks"}
                </button>
                <div class="grid col-span-2">
                    <NetworkScanProgress {networks} {isScanning} />
                </div>

                <NetworkList
                    {networks}
                    on:selectNetwork={(e) => selectNetwork(e.detail)}
                />
                <PacketSniffer />
            </div>
        {/if}
    </div>
</main>

<style>
    .analyzer {
        @apply flex flex-col justify-between mb-5;
    }
    .network-details-container {
        @apply flex flex-col items-center w-full;
    }
    .back-button {
        @apply p-2 text-lg bg-gray-700 hover:bg-gray-600 text-white rounded mb-4;
    }
    :global(body) {
        @apply bg-gray-800 text-white;
    }
</style>
