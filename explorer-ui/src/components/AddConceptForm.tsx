// In src/components/AddConceptForm.tsx
import React, { useState } from 'react';
import axios from 'axios';
// This defines the "props" or inputs our component will accept.
// It needs a function to call when the form is successfully submitted.
interface AddConceptFormProps {
  onConceptAdded: () => void;
}

const AddConceptForm: React.FC<AddConceptFormProps> = ({ onConceptAdded }) => {
  // Create state variables for the form input and any messages.
  const [name, setName] = useState('');
  const [statusMessage, setStatusMessage] = useState('');
  const [isLoading, setIsLoading] = useState(false);

  const handleSubmit = async (event: React.FormEvent) => {
    event.preventDefault(); // Prevent the browser from doing a full page reload.

    if (!name) {
      setStatusMessage('Please enter a name.');
      return;
    }

    setIsLoading(true);
    setStatusMessage('Creating concept...');

    try {
      const backendUrl = import.meta.env.VITE_BACKEND_URL || 'http://localhost:8080';
      const apiClient = axios.create({ baseURL: backendUrl });

      // Send the POST request to our Rust backend.
      await apiClient.post('/concepts', {
        data: {
          type: 'person',
          name: name, // Use the name from the input field
        }
      });

      setStatusMessage(`Successfully created concept: ${name}!`);
      setName(''); // Clear the input field.
      onConceptAdded(); // Tell the parent component to refresh the graph.

    } catch (error) {
      console.error("Failed to create concept:", error);
      setStatusMessage('Error: Could not create concept.');
    } finally {
      setIsLoading(false);
    }
  };

  return (
    <div style={{ margin: '20px', padding: '20px', border: '1px solid #ccc' }}>
      <h3>Add a New Person</h3>
      <form onSubmit={handleSubmit}>
        <input
          type="text"
          value={name}
          onChange={(e) => setName(e.target.value)}
          placeholder="Enter person's name"
          disabled={isLoading}
          style={{ marginRight: '10px' }}
        />
        <button type="submit" disabled={isLoading}>
          {isLoading ? 'Creating...' : 'Create Concept'}
        </button>
      </form>
      {statusMessage && <p>{statusMessage}</p>}
    </div>
  );
};

export default AddConceptForm;