// In explorer-ui/src/App.tsx
import { useState } from 'react';
import GraphCanvas from './components/GraphCanvas';
import AddConceptForm from './components/AddConceptForm';

function App() {
  // Create a state variable that we can change to force a refresh.
  const [graphKey, setGraphKey] = useState(0);

  const handleGraphUpdate = () => {
    // Incrementing the key will cause the GraphCanvas to unmount and re-mount,
    // which will trigger its data fetching `useEffect` hook again.
    setGraphKey(prevKey => prevKey + 1);
  };

  return (
    <div style={{ fontFamily: 'sans-serif' }}>
      <h1>Mnemonic Explorer</h1>
      
      {/* Pass the refresh function to our form */}
      <AddConceptForm onConceptAdded={handleGraphUpdate} />

      <hr />
      
      {/* Add the key prop to our GraphCanvas */}
      <GraphCanvas key={graphKey} />
    </div>
  );
}

export default App;