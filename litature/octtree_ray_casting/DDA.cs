/// <summary>
/// draw a 3D DDA "line" in units of leaf size where the ray intersects the
/// tree's bounding volume/
/// </summary>
/// <param name="ray"></param>
public IEnumerable<Vector3> DDA(Ray ray)
{

    float tmin;
    float tmax;


    if (!RayCasting.HitsBox(ray, root.BoundingBox.Min, root.BoundingBox.Max, out tmin, out tmax))
        yield break;

    /// move the ray position to the point of intersection with the bounding volume.
    ray.Position += ray.Direction * tmin;

    /// get integer cell coordinates for the given position
    var cell = Vector3.Min(((ray.Position - boundingBox.Min) / leafSize).Truncate(), cellCount - Vector3.One);

    /// get the bounds of the starting cell.
    var cellBounds = GetCellBounds(cell);

    /// calculate initial t values for each axis based on the sign of the ray.
    var tMaxNeg = (cellBounds.Min - ray.Position) / ray.Direction;
    var tMaxPos = (cellBounds.Max - ray.Position) / ray.Direction;

    /// calculate t values within the cell along the ray direction.
    var tMax = new Vector3(
        ray.Direction.X < 0 ? tMaxNeg.X : tMaxPos.X
        ,
        ray.Direction.Y < 0 ? tMaxNeg.Y : tMaxPos.Y
        ,
        ray.Direction.Z < 0 ? tMaxNeg.Z : tMaxPos.Z
        );

    /// get cell coordinate step directions
    var step = ray.Direction.Sign();

    /// calculate distance along the ray direction to move to advance from one cell boundary 
    /// to the next on each axis. Assumes ray.Direction is normalized.
    var tDelta = (leafSize / ray.Direction).Abs();

    /// step across the bounding volume, generating a marker entity at each
    /// cell that we touch. Extension methods GreaterThanOrEEqual and LessThan
    /// ensure that we stay within the bounding volume.
    while (cell.GreaterThanOrEqual(Vector3.Zero) && cell.LessThan(cellCount))
    {
        yield return boundingBox.Min + cell * leafSize;
        ///create a cube at the given cell coordinates, and add it to the draw list.
        if (tMax.X < tMax.Y)
        {
            if (tMax.X < tMax.Z)
            {
                tMax.X += tDelta.X;
                cell.X += step.X;
            }
            else
            {
                tMax.Z += tDelta.Z;
                cell.Z += step.Z;
            }
        }
        else
        {
            if (tMax.Y < tMax.Z)
            {
                tMax.Y += tDelta.Y;
                cell.Y += step.Y;

            }
            else
            {
                tMax.Z += tDelta.Z;
                cell.Z += step.Z;
            }
        }
    }

}