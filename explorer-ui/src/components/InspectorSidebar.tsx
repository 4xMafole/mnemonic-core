// In explorer-ui/src/components/InspectorSidebar.tsx
import { useEffect, useState } from 'react';
import axios from 'axios';

// The component takes an optional conceptId. If null, it shows nothing.
interface InspectorSidebarProps {
    conceptId: string | null;
}

const InspectorSidebar: React.FC<InspectorSidebarProps> = ({ conceptId }) => {
    // We will define a more specific 'Concept' type later.
    const [concept, setConcept] = useState<any>(null);
    const [isLoading, setIsLoading] = useState(false);

    // This effect runs whenever the selected `conceptId` prop changes.
    useEffect(() => {
        if (!conceptId) {
            setConcept(null);
            return;
        }

        const fetchConcept = async () => {
            setIsLoading(true);
            try {
                const backendUrl = import.meta.env.VITE_BACKEND_URL || 'http://localhost:8080';
                const response = await axios.get(`${backendUrl}/concepts/${conceptId}`);
                setConcept(response.data);
            } catch (error) {
                console.error("Failed to fetch concept details:", error);
                setConcept(null);
            } finally {
                setIsLoading(false);
            }
        };

        fetchConcept();
    }, [conceptId]);

    // Simple styling for our sidebar.
    const style: React.CSSProperties = {
        width: '300px',
        padding: '15px',
        borderLeft: '1px solid #ccc',
        position: 'absolute',
        right: 0,
        top: 0,
        bottom: 0,
        background: '#f9f9f9',
    };

    if (!conceptId) {
        return <div style={style}><p>Click on a node to inspect its data.</p></div>;
    }

    if (isLoading) {
        return <div style={style}><p>Loading details for {conceptId}...</p></div>;
    }

    return (
        <div style={style}>
            <h3>Inspecting Concept</h3>
            <p><strong>ID:</strong> {concept?.id}</p>
            <hr />
            <h4>Data:</h4>
            {/* New, safe block */}
            <pre style={{ whiteSpace: 'pre-wrap', wordBreak: 'break-all', background: '#eee', padding: '10px', borderRadius: '5px' }}>
                {
                    // --- THIS IS THE FIX ---
                    // 1. Check if the 'Structured' field exists and has content.
                    (concept?.data && concept.data.Structured)
                        // 2. If it does, parse and stringify it nicely.
                        ? JSON.stringify(JSON.parse(concept.data.Structured), null, 2)
                        // 3. Otherwise, check if it's the 'Empty' variant.
                        : (concept?.data && typeof concept.data.Empty !== 'undefined')
                            ? "(No data - Empty Concept)"
                            // 4. Fallback for any other unexpected format.
                            : "Invalid data format"
                }
            </pre>
        </div>
    );
};

export default InspectorSidebar;