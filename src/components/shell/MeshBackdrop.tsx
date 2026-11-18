import React from "react";

export const MeshBackdrop: React.FC = () => {
  return (
    <div className="mesh-backdrop" aria-hidden="true">
      <div className="mesh-blob mesh-blob-a" />
      <div className="mesh-blob mesh-blob-b" />
      <div className="mesh-blob mesh-blob-c" />
    </div>
  );
};
