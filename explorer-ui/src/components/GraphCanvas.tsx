// In explorer-ui/src/components/GraphCanvas.tsx

import React, { useEffect, useState, useRef } from 'react';
import axios from 'axios';
import * as d3 from 'd3';
import type { GraphData, GraphNode } from '../types';

// --- Define the props this component accepts ---
interface GraphCanvasProps {
  onNodeClick: (nodeId: string) => void;
}

// --- The D3 Visualization Logic (Now includes onNodeClick) ---
const drawGraph = (
    svgElement: SVGSVGElement,
    data: GraphData,
    onNodeClick: (nodeId: string) => void // It needs to know about the click handler
) => {
    const svg = d3.select(svgElement);
    svg.selectAll("*").remove();

    const width = 800;
    const height = 600;

    const simulation = d3.forceSimulation(data.nodes as d3.SimulationNodeDatum[])
        .force("link", d3.forceLink(data.edges).id(d => (d as GraphNode).id).distance(100))
        .force("charge", d3.forceManyBody().strength(-200))
        .force("center", d3.forceCenter(width / 2, height / 2));

    const link = svg.append("g")
        .selectAll("line")
        .data(data.edges)
        .join("line")
        .attr("stroke", "#999")
        .attr("stroke-opacity", 0.6);

    // --- THIS IS THE CRUCIAL CHANGE IN THE DRAWING LOGIC ---
    const node = svg.append("g")
        .selectAll("circle")
        .data(data.nodes)
        .join("circle")
        .attr("r", 10)
        .attr("fill", "#69b3a2")
        .style("cursor", "pointer") // Add a pointer cursor to indicate it's clickable
        .on("click", (event, d_node) => {
            // d_node is the specific node that was clicked.
            event.stopPropagation(); // Prevents the click from bubbling up
            onNodeClick(d_node.id);  // Call the function passed in via props
        })
        .call(drag(simulation) as any);

    const label = svg.append("g")
        .selectAll("text")
        .data(data.nodes)
        .join("text")
        .text(d => d.label)
        .attr("x", 12)
        .attr("y", 4)
        .style("font-size", "12px")
        .style("fill", "#333")
        .style("pointer-events", "none"); // Makes text non-clickable

    simulation.on("tick", () => {
        link
            .attr("x1", d => (d.source as any).x)
            .attr("y1", d => (d.source as any).y)
            .attr("x2", d => (d.target as any).x)
            .attr("y2", d => (d.target as any).y);

        node
            .attr("cx", d => (d as any).x)
            .attr("cy", d => (d as any).y);
            
        label
            .attr("x", d => (d as any).x + 12)
            .attr("y", d => (d as any).y + 4);
    });

    function drag(simulation: d3.Simulation<d3.SimulationNodeDatum, undefined>) {
        function dragstarted(event: any) {
            if (!event.active) simulation.alphaTarget(0.3).restart();
            event.subject.fx = event.subject.x;
            event.subject.fy = event.subject.y;
        }
        function dragged(event: any) {
            event.subject.fx = event.x;
            event.subject.fy = event.y;
        }
        function dragended(event: any) {
            if (!event.active) simulation.alphaTarget(0);
            event.subject.fx = null;
            event.subject.fy = null;
        }
        return d3.drag().on("start", dragstarted).on("drag", dragged).on("end", dragended);
    }
};


// --- The Main React Component (Now correctly accepts and uses the props) ---
const GraphCanvas: React.FC<GraphCanvasProps> = ({ onNodeClick }) => {
    const svgRef = useRef<SVGSVGElement | null>(null);
    const [graphData, setGraphData] = useState<GraphData>({ nodes: [], edges: [] });
    const [status, setStatus] = useState<string>('Loading...');

    // This useEffect hook handles fetching the data. (Unchanged)
    useEffect(() => {
        const fetchGraphData = async () => {
            try {
                const backendUrl = import.meta.env.VITE_BACKEND_URL || 'http://localhost:8080';
                const response = await axios.get<GraphData>(`${backendUrl}/graph`);
                setGraphData(response.data);
                setStatus('Data loaded successfully.');
            } catch (error) {
                console.error("Failed to fetch graph data:", error);
                setStatus('Failed to load data. Is the backend server running?');
            }
        };
        fetchGraphData();
    }, []);

    // This useEffect hook handles DRAWING the data. (Now passes onNodeClick)
    useEffect(() => {
        if (svgRef.current && (graphData.nodes.length > 0 || graphData.edges.length > 0)) {
            // When we call drawGraph, we now pass it the onNodeClick handler.
            drawGraph(svgRef.current, graphData, onNodeClick);
        }
    }, [graphData, onNodeClick]); // Add onNodeClick to dependency array

    return (
        <div>
            <h2>Graph Visualization</h2>
            <p>Status: {status}</p>
            <svg ref={svgRef} width="800" height="600" style={{ border: '1px solid black' }}></svg>
        </div>
    );
};

export default GraphCanvas;