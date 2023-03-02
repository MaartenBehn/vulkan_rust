/// https://stackoverflow.com/questions/6281415/octree-raycasting-raytracing-best-ray-leaf-intersection-without-recursion

/// <param name="ray"></param>
public OctreeNode DDATraverse(Ray ray)
{
    float tmin;
    float tmax;


    /// make sure the ray hits the bounding box of the root octree node
    if (!RayCasting.HitsBox(ray, root.BoundingBox.Min, root.BoundingBox.Max, out tmin, out tmax))
        return null;


    /// move the ray position to the point of intersection with the bounding volume.
    ray.Position += ray.Direction * MathHelper.Min(tmin, tmax);// intersectionDistance.Value;

    /// get integer cell coordinates for the given position
    ///     leafSize is a Vector3 containing the dimensions of a leaf node in world-space coordinates
    ///     cellCount is a Vector3 containng the number of cells in each direction, or the size of the tree root divided by leafSize.

    var cell = Vector3.Min(((ray.Position - boundingBox.Min) / leafSize).Truncate(), cellCount - Vector3.One);

    /// get the Vector3 where of the intersection point relative to the tree root.
    var pos = ray.Position - boundingBox.Min;

    /// get the bounds of the starting cell - leaf size offset by "pos"
    var cellBounds = GetCellBounds(cell);

    /// calculate initial t values for each axis based on the sign of the ray.
    /// See any good 3D DDA tutorial for an explanation of t, but it basically tells us the 
    /// distance we have to move from ray.Position along ray.Direction to reach the next cell boundary
    /// This calculates t values for both positive and negative ray directions.
    var tMaxNeg = (cellBounds.Min - ray.Position) / ray.Direction;
    var tMaxPos = (cellBounds.Max - ray.Position) / ray.Direction;

    /// calculate t values within the cell along the ray direction.
    /// This may be buggy as it seems odd to mix and match ray directions
    var tMax = new Vector3(
        ray.Direction.X < 0 ? tMaxNeg.X : tMaxPos.X
        ,
        ray.Direction.Y < 0 ? tMaxNeg.Y : tMaxPos.Y
        ,
        ray.Direction.Z < 0 ? tMaxNeg.Z : tMaxPos.Z
        );

    /// get cell coordinate step directions
    /// .Sign() is an extension method that returns a Vector3 with each component set to +/- 1
    var step = ray.Direction.Sign();

    /// calculate distance along the ray direction to move to advance from one cell boundary 
    /// to the next on each axis. Assumes ray.Direction is normalized.
    /// Takes the absolute value of each ray component since this value is in units along the
    /// ray direction, which makes sure the sign is correct.
    var tDelta = (leafSize / ray.Direction).Abs();

    /// neighbor node indices to use when exiting cells
    /// GridDirection.East = Vector3.Right
    /// GridDirection.West = Vector3.Left
    /// GridDirection.North = Vector3.Forward
    /// GridDirection.South = Vector4.Back
    /// GridDirection.Up = Vector3.Up
    /// GridDirection.Down = Vector3.Down
    var neighborDirections = new[] { 
        (step.X < 0) ? GridDirection.West : GridDirection.East
        ,
        (step.Y < 0) ? GridDirection.Down : GridDirection.Up
        ,
        (step.Z < 0) ? GridDirection.North : GridDirection.South
    };



    OctreeNode node=root;

    while (node!=null)
    {
        /// if the current node isn't a leaf, find one.
        /// this version exits when it encounters the first leaf.
        if (!node.Leaf)
            for (var i = 0; i < OctreeNode.ChildCount; i++)
            {
                var child = node.Children[i];
                if (child != null && child.Contains(cell))
                {
                    //SetNode(ref node, child, visitedNodes);
                    node = child;
                    i = -1;

                    if (node.Leaf)
                        return node;
                }
            }

        /// index into the node's Neighbor[] array to move
        int dir = 0;

        /// This is off-the-shelf DDA.
        if (tMax.X < tMax.Y)
        {
            if (tMax.X < tMax.Z)
            {
                tMax.X += tDelta.X;
                cell.X += step.X;
                dir = 0;

            }
            else
            {
                tMax.Z += tDelta.Z;
                cell.Z += step.Z;
                dir = 2;

            }
        }
        else
        {
            if (tMax.Y < tMax.Z)
            {
                tMax.Y += tDelta.Y;
                cell.Y += step.Y;
                dir = 1;
            }
            else
            {
                tMax.Z += tDelta.Z;
                cell.Z += step.Z;
                dir = 2;
            }
        }

        /// see if the new cell coordinates fall within the current node.
        /// this is important when moving from a leaf into empty space within 
        /// the tree.
        if (!node.Contains(cell))
        {
            /// if we stepped out of this node, grab the appropriate neighbor. 
            var neighborDir = neighborDirections[dir];
            node = node.GetNeighbor(neighborDir);
        }
        else if (node.Leaf && stopAtFirstLeaf)
            return node;
    }

    return null;

}