// In explorer-ui/src/App.tsx
import { useState } from 'react';
import GraphCanvas from './components/GraphCanvas';
import AddConceptForm from './components/AddConceptForm';
import InspectorSidebar from './components/InspectorSidebar'; // Import the new component

function App() {
  const [graphKey, setGraphKey] = useState(0);
  // NEW STATE: Keep track of the currently selected concept ID.
  const [selectedConceptId, setSelectedConceptId] = useState<string | null>(null);

  const handleGraphUpdate = () => {
    setGraphKey(prevKey => prevKey + 1);
  };
  
  // Create a container to handle layout.
  const mainStyle: React.CSSProperties = {
      position: 'relative',
  };

  return (
    <div style={mainStyle}>
      <div style={{ marginRight: '330px' }}> {/* Add margin to prevent overlap */}
        <h1>Mnemonic Explorer</h1>
        <AddConceptForm onConceptAdded={handleGraphUpdate} />
        <hr />
        <GraphCanvas 
            key={graphKey} 
            onNodeClick={setSelectedConceptId}  // Pass the setter function (typed as any to satisfy TSX)
        />
      </div>

      <InspectorSidebar conceptId={selectedConceptId} />
    </div>
  );
}

export default App;